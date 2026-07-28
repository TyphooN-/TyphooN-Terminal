use super::*;

fn action(time: i64, symbol: &str, kind: CorporateActionKind) -> CorporateAction {
    CorporateAction {
        symbol: symbol.into(),
        effective_time_ns: time,
        kind,
    }
}

#[test]
fn schedule_build_canonicalizes_declaration_order() {
    let split = action(
        10,
        "AAA",
        CorporateActionKind::Split {
            numerator: 2,
            denominator: 1,
        },
    );
    let dividend = action(
        10,
        "AAA",
        CorporateActionKind::CashDividend {
            amount_per_unit: 1.0,
        },
    );
    let first = CorporateActionSchedule::build(&[dividend.clone(), split.clone()]).unwrap();
    let second = CorporateActionSchedule::build(&[split, dividend]).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.actions()[0].kind.wire_id(), "split");
}

#[test]
fn adjusted_prices_refuse_double_counted_events() {
    let schedule = CorporateActionSchedule::build(&[action(
        10,
        "AAA",
        CorporateActionKind::Split {
            numerator: 2,
            denominator: 1,
        },
    )])
    .unwrap();
    assert!(
        schedule
            .check_adjustment_consistency(AdjustmentPolicy::Raw)
            .is_ok()
    );
    assert!(matches!(
        schedule.check_adjustment_consistency(AdjustmentPolicy::SplitAdjusted),
        Err(CorporateActionError::DoubleCounted { .. })
    ));
}
