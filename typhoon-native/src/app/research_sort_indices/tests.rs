use std::cell::Cell;

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
