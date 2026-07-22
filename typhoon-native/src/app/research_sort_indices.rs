use std::cmp::Ordering;
use std::sync::Arc;

#[derive(Debug, Default)]
pub(crate) struct SortedRowIndices {
    indices: Arc<[usize]>,
    sort_key: Option<(usize, bool)>,
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
        mut compare: F,
    ) -> Arc<[usize]>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        let sort_key = (sort_column, ascending);
        if !self.valid || self.sort_key != Some(sort_key) || self.indices.len() != rows.len() {
            let mut indices: Vec<usize> = (0..rows.len()).collect();
            indices.sort_by(|&left_index, &right_index| {
                let order = compare(&rows[left_index], &rows[right_index]);
                let order = if ascending { order } else { order.reverse() };
                order.then_with(|| left_index.cmp(&right_index))
            });
            self.indices = indices.into();
            self.sort_key = Some(sort_key);
            self.valid = true;
        }
        Arc::clone(&self.indices)
    }
}

#[cfg(test)]
mod tests;
