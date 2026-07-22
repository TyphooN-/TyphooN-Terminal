use super::prepare_omon_strike_rows;
use typhoon_engine::core::research::{OptionContract, OptionExpiry};

fn contract(strike: f64) -> OptionContract {
    OptionContract {
        strike,
        ..Default::default()
    }
}

#[test]
fn prepared_rows_preserve_legacy_duplicate_and_last_contract_semantics() {
    let expiry = OptionExpiry {
        calls: vec![contract(105.0), contract(100.0), contract(100.0)],
        puts: vec![
            contract(95.0),
            contract(100.0),
            contract(100.0),
            contract(110.0),
        ],
        ..Default::default()
    };

    let rows = prepare_omon_strike_rows(&expiry);
    let actual: Vec<_> = rows
        .iter()
        .map(|row| (row.strike, row.call_index, row.put_index))
        .collect();

    assert_eq!(
        actual,
        vec![
            (95.0, None, Some(0)),
            (100.0, Some(2), Some(2)),
            (100.0, Some(2), Some(2)),
            (105.0, Some(0), None),
            (110.0, None, Some(3)),
        ]
    );
}

#[test]
fn prepared_rows_sort_put_only_expiry_and_handle_empty_expiry() {
    let puts_only = OptionExpiry {
        puts: vec![contract(110.0), contract(90.0), contract(100.0)],
        ..Default::default()
    };

    let rows = prepare_omon_strike_rows(&puts_only);
    assert_eq!(
        rows.iter().map(|row| row.strike).collect::<Vec<_>>(),
        vec![90.0, 100.0, 110.0]
    );
    assert!(rows.iter().all(|row| row.call_index.is_none()));
    assert!(prepare_omon_strike_rows(&OptionExpiry::default()).is_empty());
}
