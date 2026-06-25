//! Broker runtime sync budgets shared by native broker orchestration and future
//! broker-runtime crate extraction.

/// Concurrent public Kraken REST fetches for spot/futures market-data tasks.
pub const KRAKEN_PUBLIC_FETCH_PERMITS: usize = 24;

/// Concurrent Kraken Securities/iapi history fetches.
pub const KRAKEN_EQUITIES_FETCH_PERMITS: usize = 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kraken_fetch_permits_match_runtime_budget() {
        assert_eq!(KRAKEN_PUBLIC_FETCH_PERMITS, 24);
        assert_eq!(KRAKEN_EQUITIES_FETCH_PERMITS, 8);
    }
}
