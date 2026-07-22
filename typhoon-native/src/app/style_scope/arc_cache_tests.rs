use std::cell::Cell;
use std::sync::Arc;

use typhoon_engine::broker::alpaca::PositionInfo;
use typhoon_engine::core::fundamentals::Fundamentals;
use typhoon_engine::core::screener::{DividendScreenEntry, SectorHeatmapEntry};

use super::{
    EventSource, broker_scope_membership_signature, position_symbol_membership_signature,
    refresh_arc_slice_cache,
};

fn position(symbol: &str) -> PositionInfo {
    PositionInfo {
        symbol: symbol.into(),
        qty: 1.0,
        qty_available: 1.0,
        side: "long".into(),
        avg_entry_price: 1.0,
        market_value: 1.0,
        unrealized_pl: 0.0,
        asset_class: "us_equity".into(),
        asset_id: symbol.into(),
    }
}

#[test]
fn unchanged_cache_key_reuses_source_and_derived_arcs_without_rebuilding() {
    let original: Arc<[Fundamentals]> = vec![Fundamentals::default()].into();
    let original_sectors: Arc<[SectorHeatmapEntry]> = vec![SectorHeatmapEntry {
        sector: "Technology".into(),
        symbol_count: 1,
        avg_change_pct: 0.0,
        total_market_cap: 1.0,
        avg_pe: 2.0,
    }]
    .into();
    let original_dividends: Arc<[DividendScreenEntry]> = vec![DividendScreenEntry {
        symbol: "AAPL".into(),
        company: "Apple".into(),
        dividend_yield: 1.0,
        ex_div_date: String::new(),
        pe_ratio: 2.0,
        market_cap: 3.0,
        is_dividend_stock: true,
    }]
    .into();
    let mut cached = Arc::clone(&original);
    let mut sectors = Arc::clone(&original_sectors);
    let mut dividends = Arc::clone(&original_dividends);
    let mut cached_key = Some((7_u64, "all"));
    let mut sectors_key = Some((7_u64, "all"));
    let mut dividends_key = Some((7_u64, "all"));
    let builds = Cell::new(0);

    let refreshed = refresh_arc_slice_cache(&mut cached, &mut cached_key, (7, "all"), || {
        builds.set(builds.get() + 1);
        Vec::new()
    });
    let sectors_refreshed =
        refresh_arc_slice_cache(&mut sectors, &mut sectors_key, (7, "all"), || {
            builds.set(builds.get() + 1);
            Vec::new()
        });
    let dividends_refreshed =
        refresh_arc_slice_cache(&mut dividends, &mut dividends_key, (7, "all"), || {
            builds.set(builds.get() + 1);
            Vec::new()
        });

    assert!(!refreshed);
    assert!(!sectors_refreshed);
    assert!(!dividends_refreshed);
    assert!(Arc::ptr_eq(&cached, &original));
    assert!(Arc::ptr_eq(&sectors, &original_sectors));
    assert!(Arc::ptr_eq(&dividends, &original_dividends));
    assert_eq!(builds.get(), 0);
}

#[test]
fn changed_cache_key_replaces_source_and_rebuilds_derived_models() {
    let original: Arc<[Fundamentals]> = vec![Fundamentals::default()].into();
    let mut cached = Arc::clone(&original);
    let mut sectors: Arc<[SectorHeatmapEntry]> = Arc::from([]);
    let mut dividends: Arc<[DividendScreenEntry]> = Arc::from([]);
    let mut cached_key = Some((7_u64, "all"));
    let mut sectors_key = Some((7_u64, "all"));
    let mut dividends_key = Some((7_u64, "all"));

    let refreshed = refresh_arc_slice_cache(&mut cached, &mut cached_key, (8, "all"), || {
        vec![
            Fundamentals {
                symbol: "AAPL".into(),
                company_name: "Apple".into(),
                sector: "Technology".into(),
                market_cap: Some(3.0),
                pe_ratio: Some(2.0),
                dividend_yield: Some(1.5),
                is_dividend_stock: true,
                ..Default::default()
            },
            Fundamentals {
                symbol: "MSFT".into(),
                sector: "Technology".into(),
                market_cap: Some(4.0),
                pe_ratio: Some(4.0),
                ..Default::default()
            },
        ]
    });
    let source = Arc::clone(&cached);
    let sectors_refreshed =
        refresh_arc_slice_cache(&mut sectors, &mut sectors_key, (8, "all"), || {
            typhoon_engine::core::screener::compute_sector_heatmap(&source)
        });
    let dividends_refreshed =
        refresh_arc_slice_cache(&mut dividends, &mut dividends_key, (8, "all"), || {
            typhoon_engine::core::screener::screen_dividend_stocks(&source)
        });

    assert!(refreshed);
    assert!(sectors_refreshed);
    assert!(dividends_refreshed);
    assert!(!Arc::ptr_eq(&cached, &original));
    assert_eq!(cached.len(), 2);
    assert_eq!(sectors.len(), 1);
    assert_eq!(sectors[0].sector, "Technology");
    assert_eq!(sectors[0].symbol_count, 2);
    assert_eq!(sectors[0].total_market_cap, 7.0);
    assert_eq!(sectors[0].avg_pe, 3.0);
    assert_eq!(dividends.len(), 1);
    assert_eq!(dividends[0].symbol, "AAPL");
    assert_eq!(dividends[0].dividend_yield, 1.5);
    assert_eq!(cached_key, Some((8, "all")));
}

#[test]
fn position_membership_change_refreshes_source_and_derived_models_without_bg_change() {
    let canonical = position_symbol_membership_signature(&[position("AAPL"), position("MSFT")]);
    let reordered_with_duplicate = position_symbol_membership_signature(&[
        position("MSFT"),
        position("AAPL"),
        position("AAPL"),
    ]);
    assert_eq!(canonical, reordered_with_duplicate);

    let aapl_signature = broker_scope_membership_signature(EventSource::Positions, 1, 0, 0);
    let msft_signature = broker_scope_membership_signature(EventSource::Positions, 2, 0, 0);
    assert_ne!(aapl_signature, msft_signature);

    let mut source: Arc<[Fundamentals]> = Arc::from([]);
    let mut sectors: Arc<[SectorHeatmapEntry]> = Arc::from([]);
    let mut dividends: Arc<[DividendScreenEntry]> = Arc::from([]);
    let mut source_key = None;
    let mut sectors_key = None;
    let mut dividends_key = None;
    let aapl_key = (7_u64, EventSource::Positions, aapl_signature);

    refresh_arc_slice_cache(&mut source, &mut source_key, aapl_key, || {
        vec![Fundamentals {
            symbol: "AAPL".into(),
            company_name: "Apple".into(),
            sector: "Technology".into(),
            market_cap: Some(3.0),
            pe_ratio: Some(2.0),
            dividend_yield: Some(1.5),
            is_dividend_stock: true,
            ..Default::default()
        }]
    });
    let scoped = Arc::clone(&source);
    refresh_arc_slice_cache(&mut sectors, &mut sectors_key, aapl_key, || {
        typhoon_engine::core::screener::compute_sector_heatmap(&scoped)
    });
    refresh_arc_slice_cache(&mut dividends, &mut dividends_key, aapl_key, || {
        typhoon_engine::core::screener::screen_dividend_stocks(&scoped)
    });
    let aapl_source = Arc::clone(&source);
    let aapl_sectors = Arc::clone(&sectors);
    let aapl_dividends = Arc::clone(&dividends);

    let msft_key = (7_u64, EventSource::Positions, msft_signature);
    refresh_arc_slice_cache(&mut source, &mut source_key, msft_key, || {
        vec![Fundamentals {
            symbol: "MSFT".into(),
            sector: "Technology".into(),
            market_cap: Some(4.0),
            pe_ratio: Some(4.0),
            ..Default::default()
        }]
    });
    let scoped = Arc::clone(&source);
    refresh_arc_slice_cache(&mut sectors, &mut sectors_key, msft_key, || {
        typhoon_engine::core::screener::compute_sector_heatmap(&scoped)
    });
    refresh_arc_slice_cache(&mut dividends, &mut dividends_key, msft_key, || {
        typhoon_engine::core::screener::screen_dividend_stocks(&scoped)
    });

    assert!(!Arc::ptr_eq(&source, &aapl_source));
    assert!(!Arc::ptr_eq(&sectors, &aapl_sectors));
    assert!(!Arc::ptr_eq(&dividends, &aapl_dividends));
    assert_eq!(source[0].symbol, "MSFT");
    assert_eq!(sectors[0].symbol_count, 1);
    assert!(dividends.is_empty());
}
