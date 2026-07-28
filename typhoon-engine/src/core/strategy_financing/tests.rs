use super::*;

fn declared(percent: f64) -> RateSource {
    RateSource::Declared {
        percent,
        provenance: RateProvenance::OperatorAssumption {
            note: "focused test".into(),
        },
    }
}

#[test]
fn accrual_uses_declared_units_and_refuses_missing_borrow() {
    let policy = FinancingPolicy {
        day_count: DayCount::Act365Fixed,
        accrual: AccrualInterval::UtcDaily,
        long_financing_annual_percent: declared(36.5),
        short_financing_annual_percent: RateSource::NotApplicable,
        short_borrow_annual_percent: RateSource::Unavailable {
            reason: "no feed".into(),
        },
        funding_interval_percent: declared(0.1),
    };
    policy.validate().unwrap();
    let long = accrue(&policy, 100.0, 10.0, 86_400).unwrap();
    assert_eq!(long.financing, 1.0);
    assert_eq!(long.borrow, 0.0);
    assert_eq!(long.funding, 1.0);
    assert_eq!(
        accrue(&policy, -100.0, 10.0, 86_400),
        Err(FinancingCharge::ShortBorrow)
    );
}

#[test]
fn conversion_rows_must_be_canonical_and_account_currency_is_implicit() {
    let provenance = RateProvenance::OperatorAssumption {
        note: "test table".into(),
    };
    let conversion = CurrencyConversion::Declared {
        rates: vec![
            CurrencyRate {
                currency: "EUR".into(),
                account_per_unit: 1.2,
                spread_percent: 0.1,
                provenance: provenance.clone(),
            },
            CurrencyRate {
                currency: "JPY".into(),
                account_per_unit: 0.007,
                spread_percent: 0.2,
                provenance,
            },
        ],
    };
    conversion.validate("USD").unwrap();
    assert_eq!(conversion.lookup("USD", "USD"), Some((1.0, 0.0)));
    assert_eq!(conversion.lookup("EUR", "USD"), Some((1.2, 0.1)));
}
