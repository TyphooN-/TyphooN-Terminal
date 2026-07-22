use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::PathBuf;

use typhoon_engine::core::fundamentals::Fundamentals;

use super::*;

#[derive(Debug)]
struct Row {
    key: i32,
    label: &'static str,
}

#[test]
fn sorted_indices_preserve_source_order_for_equal_keys_in_both_directions() {
    let rows = [
        Row { key: 2, label: "a" },
        Row { key: 1, label: "b" },
        Row { key: 2, label: "c" },
    ];
    let mut cache = SortedRowIndices::default();

    let ascending = cache.order(&rows, 0, true, |a, b| a.key.cmp(&b.key));
    let descending = cache.order(&rows, 0, false, |a, b| a.key.cmp(&b.key));

    assert_eq!(ascending.as_ref(), [1, 0, 2]);
    assert_eq!(descending.as_ref(), [0, 2, 1]);
    assert_eq!(rows[descending[0]].label, "a");
    assert_eq!(rows[descending[1]].label, "c");
}

#[test]
fn reverse_after_sort_mode_preserves_legacy_descending_tie_order() {
    let rows = [
        Row { key: 2, label: "a" },
        Row { key: 1, label: "b" },
        Row { key: 2, label: "c" },
    ];
    let mut cache = SortedRowIndices::default();

    let descending =
        cache.order_then_reverse(&rows, 0, false, |left, right| left.key.cmp(&right.key));

    assert_eq!(descending.as_ref(), [2, 0, 1]);
    assert_eq!(rows[descending[0]].label, "c");
    assert_eq!(rows[descending[1]].label, "a");
}

#[test]
fn sorted_indices_reuse_matching_sort_and_rebuild_when_sort_changes() {
    let rows = [Row { key: 2, label: "a" }, Row { key: 1, label: "b" }];
    let comparisons = Cell::new(0);
    let mut cache = SortedRowIndices::default();
    let compare = |left: &Row, right: &Row| {
        comparisons.set(comparisons.get() + 1);
        left.key.cmp(&right.key)
    };

    assert_eq!(cache.order(&rows, 0, true, compare).as_ref(), [1, 0]);
    let after_first_build = comparisons.get();
    assert!(after_first_build > 0);
    assert_eq!(cache.order(&rows, 0, true, compare).as_ref(), [1, 0]);
    assert_eq!(comparisons.get(), after_first_build);
    assert_eq!(cache.order(&rows, 0, false, compare).as_ref(), [0, 1]);
    assert!(comparisons.get() > after_first_build);
}

#[test]
fn invalidation_rebuilds_same_length_replacements_without_stale_indices() {
    let old_rows = [Row { key: 1, label: "a" }, Row { key: 2, label: "b" }];
    let new_rows = [Row { key: 4, label: "c" }, Row { key: 3, label: "d" }];
    let mut cache = SortedRowIndices::default();

    assert_eq!(
        cache
            .order(&old_rows, 0, true, |a, b| a.key.cmp(&b.key))
            .as_ref(),
        [0, 1]
    );
    cache.invalidate();
    assert_eq!(
        cache
            .order(&new_rows, 0, true, |a, b| a.key.cmp(&b.key))
            .as_ref(),
        [1, 0]
    );
}

#[test]
fn row_count_changes_rebuild_without_out_of_bounds_indices() {
    let mut cache = SortedRowIndices::default();
    let populated = [Row { key: 2, label: "a" }, Row { key: 1, label: "b" }];

    assert!(
        cache
            .order::<Row, _>(&[], 0, true, |a, b| a.key.cmp(&b.key))
            .is_empty()
    );
    assert_eq!(
        cache
            .order(&populated, 0, true, |a, b| a.key.cmp(&b.key))
            .as_ref(),
        [1, 0]
    );
    assert!(
        cache
            .order::<Row, _>(&[], 0, true, |a, b| a.key.cmp(&b.key))
            .is_empty()
    );
}

#[test]
fn fundamentals_order_preserves_all_ev_scanner_column_semantics() {
    let left = Fundamentals {
        symbol: "AAA".into(),
        company_name: "Alpha".into(),
        enterprise_value: Some(1.0),
        market_cap: Some(2.0),
        mcap_ev_ratio: Some(3.0),
        pe_ratio: Some(4.0),
        next_earnings_date: Some("2026-01-01".into()),
        dividend_yield: Some(5.0),
        sector: "Energy".into(),
        ..Default::default()
    };
    let right = Fundamentals {
        symbol: "ZZZ".into(),
        company_name: "Zulu".into(),
        enterprise_value: Some(11.0),
        market_cap: Some(12.0),
        mcap_ev_ratio: Some(13.0),
        pe_ratio: Some(14.0),
        next_earnings_date: Some("2026-12-31".into()),
        dividend_yield: Some(15.0),
        sector: "Technology".into(),
        ..Default::default()
    };

    for column in 0..=8 {
        assert_eq!(fundamentals_order(&left, &right, column), Ordering::Less);
    }
    assert_eq!(fundamentals_order(&left, &right, 99), Ordering::Equal);
}

#[test]
fn screenshot_order_preserves_file_size_and_taken_columns() {
    let left = (PathBuf::from("a.webp"), 10_i64, 20_u64);
    let right = (PathBuf::from("z.webp"), 30_i64, 40_u64);

    assert_eq!(screenshot_order(&left, &right, 0), Ordering::Less);
    assert_eq!(screenshot_order(&left, &right, 1), Ordering::Less);
    assert_eq!(screenshot_order(&left, &right, 2), Ordering::Less);
    assert_eq!(screenshot_order(&left, &right, 99), Ordering::Less);
}

#[test]
fn active_fundamental_match_uses_direct_and_normalized_symbols() {
    let active = HashSet::from(["AAPL".to_string(), "WOK".to_string()]);

    assert!(fundamental_matches_active_set("AAPL", &active));
    assert!(fundamental_matches_active_set("WOK.EQ", &active));
    assert!(!fundamental_matches_active_set("MSFT", &active));
}
