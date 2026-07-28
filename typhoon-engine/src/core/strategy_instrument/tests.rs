use super::*;
use crate::core::strategy_financing::{CurrencyConversion, CurrencyRate, RateProvenance};

#[test]
fn registry_build_is_canonical_and_lookup_is_deterministic() {
    let first = InstrumentRegistry::build(&[
        InstrumentSpec::plain("ZZZ", "USD"),
        InstrumentSpec::plain("AAA", "USD"),
    ])
    .unwrap();
    let second = InstrumentRegistry::build(&[
        InstrumentSpec::plain("AAA", "USD"),
        InstrumentSpec::plain("ZZZ", "USD"),
    ])
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.specs()[0].symbol, "AAA");
    assert_eq!(first.get("ZZZ").unwrap().currency, "USD");
}

#[test]
fn foreign_instrument_requires_an_explicit_conversion() {
    let registry = InstrumentRegistry::build(&[InstrumentSpec::plain("SAP", "EUR")]).unwrap();
    assert!(matches!(
        registry.validate_against_account("USD", &CurrencyConversion::None),
        Err(InstrumentError::UnconvertibleCurrency { .. })
    ));
    let conversion = CurrencyConversion::Declared {
        rates: vec![CurrencyRate {
            currency: "EUR".into(),
            account_per_unit: 1.2,
            spread_percent: 0.0,
            provenance: RateProvenance::OperatorAssumption {
                note: "test".into(),
            },
        }],
    };
    assert!(
        registry
            .validate_against_account("USD", &conversion)
            .is_ok()
    );
}
