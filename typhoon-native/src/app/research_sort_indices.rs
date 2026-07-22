use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use typhoon_engine::broker::cache_keys::bare_symbol_from_key;
use typhoon_engine::core::fundamentals::Fundamentals;

pub(crate) fn fundamentals_order(
    left: &Fundamentals,
    right: &Fundamentals,
    column: usize,
) -> Ordering {
    match column {
        0 => left.symbol.cmp(&right.symbol),
        1 => left.company_name.cmp(&right.company_name),
        2 => left
            .enterprise_value
            .unwrap_or(0.0)
            .partial_cmp(&right.enterprise_value.unwrap_or(0.0))
            .unwrap_or(Ordering::Equal),
        3 => left
            .market_cap
            .unwrap_or(0.0)
            .partial_cmp(&right.market_cap.unwrap_or(0.0))
            .unwrap_or(Ordering::Equal),
        4 => left
            .mcap_ev_ratio
            .unwrap_or(0.0)
            .partial_cmp(&right.mcap_ev_ratio.unwrap_or(0.0))
            .unwrap_or(Ordering::Equal),
        5 => left
            .pe_ratio
            .unwrap_or(0.0)
            .partial_cmp(&right.pe_ratio.unwrap_or(0.0))
            .unwrap_or(Ordering::Equal),
        6 => left
            .next_earnings_date
            .as_deref()
            .unwrap_or("")
            .cmp(right.next_earnings_date.as_deref().unwrap_or("")),
        7 => left
            .dividend_yield
            .unwrap_or(0.0)
            .partial_cmp(&right.dividend_yield.unwrap_or(0.0))
            .unwrap_or(Ordering::Equal),
        8 => left.sector.cmp(&right.sector),
        _ => Ordering::Equal,
    }
}

pub(crate) fn screenshot_order(
    left: &(PathBuf, i64, u64),
    right: &(PathBuf, i64, u64),
    column: usize,
) -> Ordering {
    match column {
        0 => left
            .0
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .cmp(
                right
                    .0
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(""),
            ),
        1 => left.2.cmp(&right.2),
        _ => left.1.cmp(&right.1),
    }
}

pub(crate) fn fundamental_matches_active_set(symbol: &str, active: &HashSet<String>) -> bool {
    if active.contains(symbol) {
        return true;
    }
    let bare = bare_symbol_from_key(symbol).to_uppercase();
    let normalized = match bare.rsplit_once('.') {
        Some((head, suffix))
            if (2..=4).contains(&suffix.len())
                && suffix
                    .chars()
                    .all(|character| character.is_ascii_uppercase()) =>
        {
            head
        }
        _ => bare.as_str(),
    };
    if normalized.contains('/') {
        active.contains(&normalized.replace('/', ""))
    } else {
        active.contains(normalized)
    }
}

#[derive(Debug, Default)]
pub(crate) struct SortedRowIndices {
    indices: Arc<[usize]>,
    sort_key: Option<(usize, bool, bool)>,
    valid: bool,
}

impl SortedRowIndices {
    pub(crate) fn invalidate(&mut self) {
        self.valid = false;
    }

    pub(crate) fn order<T, F>(
        &mut self,
        rows: &[T],
        sort_column: usize,
        ascending: bool,
        compare: F,
    ) -> Arc<[usize]>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.order_impl(rows, sort_column, ascending, false, compare)
    }

    pub(crate) fn order_then_reverse<T, F>(
        &mut self,
        rows: &[T],
        sort_column: usize,
        ascending: bool,
        compare: F,
    ) -> Arc<[usize]>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.order_impl(rows, sort_column, ascending, true, compare)
    }

    fn order_impl<T, F>(
        &mut self,
        rows: &[T],
        sort_column: usize,
        ascending: bool,
        reverse_after_sort: bool,
        mut compare: F,
    ) -> Arc<[usize]>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        let sort_key = (sort_column, ascending, reverse_after_sort);
        if !self.valid || self.sort_key != Some(sort_key) || self.indices.len() != rows.len() {
            let mut indices: Vec<usize> = (0..rows.len()).collect();
            if reverse_after_sort {
                indices.sort_by(|&left_index, &right_index| {
                    compare(&rows[left_index], &rows[right_index])
                });
                if !ascending {
                    indices.reverse();
                }
            } else {
                indices.sort_by(|&left_index, &right_index| {
                    let order = compare(&rows[left_index], &rows[right_index]);
                    let order = if ascending { order } else { order.reverse() };
                    order.then_with(|| left_index.cmp(&right_index))
                });
            }
            self.indices = indices.into();
            self.sort_key = Some(sort_key);
            self.valid = true;
        }
        Arc::clone(&self.indices)
    }
}

#[cfg(test)]
mod tests;
