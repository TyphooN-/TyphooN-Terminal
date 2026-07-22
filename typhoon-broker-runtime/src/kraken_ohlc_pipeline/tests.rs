use super::*;

#[test]
fn pair_universe_clones_share_the_same_backing_allocation() {
    let universe = share_pair_universe(vec!["BTC/USD".to_string(), "ETH/USD".to_string()]);
    let interval_clone = universe.clone();

    assert!(Arc::ptr_eq(&universe, &interval_clone));
    assert_eq!(
        &*interval_clone,
        &["BTC/USD".to_string(), "ETH/USD".to_string()]
    );
}
