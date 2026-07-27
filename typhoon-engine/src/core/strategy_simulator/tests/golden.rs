// ── Golden corpus (ADR-135 §13 M1 gate clause 1) ───────────────────
//
// Every number below is derived by hand in the comment that precedes it, from
// the bar rows in the same test. Nothing here is a recorded snapshot of what
// the engine happened to produce: if the engine changes its mind about a fill
// price, a fee, or a cash balance, one of these numbers has to be re-derived
// on paper first.
//
// Cost conventions used throughout, all from `costed_settings()`:
//   spread   0.10 price units  → half-spread 0.05, buys lift the ask, sells
//                                hit the bid
//   slippage 0.02 price units  → applied to marketable orders only, always
//                                adverse
//   commission per share 0.01, minimum 1.00 → 10 shares costs the 1.00 floor

/// A long round trip with commission and spread, computed by hand.
///
/// Bars (open, high, low, close):
///   0: 100.0 101.0  99.0 100.5   decision: buy 10
///   1: 101.0 102.0 100.0 101.5   the buy fills at this open
///   2: 103.0 104.0 102.0 103.5   decision: sell 10
///   3: 105.0 106.0 104.0 105.5   the sell fills at this open
///
/// Buy at bar 1's open: ask = 101.00 + 0.05 = 101.05, plus 0.02 slippage
///   → fill 101.07. Fee max(0.01 × 10, 1.00) = 1.00.
///   cash = 100 000 − 101.07 × 10 − 1.00 = 98 988.30
/// Sell at bar 3's open: bid = 105.00 − 0.05 = 104.95, minus 0.02 slippage
///   → fill 104.93. Fee 1.00.
///   cash = 98 988.30 + 104.93 × 10 − 1.00 = 100 036.60
///   realized = (104.93 − 101.07) × 10 = 38.60
#[test]
fn golden_long_round_trip_with_commission_and_spread() {
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 101.0, 99.0, 100.5),
            (101.0, 102.0, 100.0, 101.5),
            (103.0, 104.0, 102.0, 103.5),
            (105.0, 106.0, 104.0, 105.5),
        ],
    );
    let mut strategy =
        ScriptedStrategy::new(vec![(0, OrderSide::Buy, 10.0), (2, OrderSide::Sell, 10.0)]);
    let report = run(costed_settings(), &[stream], &mut strategy).expect("runs");

    assert_eq!(report.fills.len(), 2);
    let entry = &report.fills[0];
    assert_eq!(entry.time_ns, MINUTE_NS);
    assert_close(entry.reference_price, 101.0, "entry reference");
    assert_close(entry.quoted_price, 101.05, "entry ask");
    assert_close(entry.fill_price, 101.07, "entry fill");
    assert_close(entry.spread_cost, 0.5, "entry spread cost");
    assert_close(entry.slippage_cost, 0.2, "entry slippage cost");
    assert_close(entry.commission, 1.0, "entry fee");
    assert_close(entry.cash_after, 98_988.30, "cash after entry");
    assert_close(entry.realized_pnl, 0.0, "an entry realizes nothing");

    let exit = &report.fills[1];
    assert_eq!(exit.time_ns, 3 * MINUTE_NS);
    assert_close(exit.quoted_price, 104.95, "exit bid");
    assert_close(exit.fill_price, 104.93, "exit fill");
    assert_close(exit.commission, 1.0, "exit fee");
    assert_close(exit.realized_pnl, 38.60, "realized");
    assert_close(exit.cash_after, 100_036.60, "cash after exit");

    assert_close(report.final_cash, 100_036.60, "final cash");
    assert_close(report.final_equity, 100_036.60, "flat, so equity is cash");
    assert_close(report.final_realized_pnl, 38.60, "final realized");
    assert_close(report.total_commission, 2.0, "total fees");
}

/// The bracket every ambiguity test shares: long 10 from bar 1's open at
/// 100.00, protected by a sell stop at 95 and a sell limit at 110 submitted on
/// bar 1's close, so both are live from bar 2's open.
fn bracket_strategy() -> OrderScript {
    OrderScript::new(vec![
        (0, OrderRequest::market(AAA, OrderSide::Buy, 10.0)),
        (
            1,
            OrderRequest::stop(AAA, OrderSide::Sell, 10.0, 95.0).with_oco(1),
        ),
        (
            1,
            OrderRequest::limit(AAA, OrderSide::Sell, 10.0, 110.0).with_oco(1),
        ),
    ])
}

/// Bar 2 reaches both 110 and 95. `close ≥ open`, so its assumed path is
/// O→H→L→C.
fn up_bar_bracket_stream() -> SymbolStream {
    stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 112.0, 94.0, 101.0),
            (101.0, 101.5, 100.5, 101.0),
        ],
    )
}

/// The same bar with `close < open`, so its assumed path is O→L→H→C.
fn down_bar_bracket_stream() -> SymbolStream {
    stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 112.0, 94.0, 99.0),
            (99.0, 99.5, 98.5, 99.0),
        ],
    )
}

/// Stop and target in the same bar, under the pessimistic default.
///
/// Entry at bar 1's open = 100.00, zero costs, so cash = 100 000 − 1 000
/// = 99 000. `StopFirst` fills the stop at 95.00:
///   cash = 99 000 + 95.00 × 10 = 99 950, realized = (95 − 100) × 10 = −50.
#[test]
fn golden_same_bar_stop_and_target_under_stop_first() {
    let mut strategy = bracket_strategy();
    let report = run(
        intrabar_settings(),
        &[up_bar_bracket_stream()],
        &mut strategy,
    )
    .expect("runs");

    assert_eq!(report.fills.len(), 2, "one entry, one protective exit");
    let exit = &report.fills[1];
    assert_close(exit.fill_price, 95.0, "the stop fills");
    assert_close(exit.realized_pnl, -50.0, "realized");
    assert_close(report.final_cash, 99_950.0, "cash");
    assert_close(report.final_equity, 99_950.0, "equity");
    assert_eq!(report.cancellations.len(), 1, "the target is cancelled");
    assert_eq!(report.cancellations[0].reason, CancelReason::OcoSibling);
}

/// The same bar under the optimistic policy: the target fills at 110.00.
///   cash = 99 000 + 110.00 × 10 = 100 100, realized = (110 − 100) × 10 = +100.
#[test]
fn golden_same_bar_stop_and_target_under_target_first() {
    let settings = ExecutionSettings {
        ambiguity: OhlcAmbiguityPolicy::TargetFirst,
        ..intrabar_settings()
    };
    let mut strategy = bracket_strategy();
    let report = run(settings, &[up_bar_bracket_stream()], &mut strategy).expect("runs");

    let exit = &report.fills[1];
    assert_close(exit.fill_price, 110.0, "the target fills");
    assert_close(exit.realized_pnl, 100.0, "realized");
    assert_close(report.final_cash, 100_100.0, "cash");
}

/// Under the OHLC-path heuristic the bar's own direction decides. On an up bar
/// the path is O→H→L→C, so 110 is reached before 94 and the target fills; on a
/// down bar the path is O→L→H→C, so the stop fills first.
#[test]
fn golden_same_bar_stop_and_target_under_the_ohlc_path() {
    let settings = ExecutionSettings {
        ambiguity: OhlcAmbiguityPolicy::OhlcPath,
        ..intrabar_settings()
    };

    let mut strategy = bracket_strategy();
    let up = run(settings.clone(), &[up_bar_bracket_stream()], &mut strategy).expect("runs");
    assert_close(
        up.fills[1].fill_price,
        110.0,
        "up bar reaches the high first",
    );
    assert_close(up.final_cash, 100_100.0, "up-bar cash");

    let mut strategy = bracket_strategy();
    let down = run(settings, &[down_bar_bracket_stream()], &mut strategy).expect("runs");
    assert_close(
        down.fills[1].fill_price,
        95.0,
        "down bar reaches the low first",
    );
    assert_close(down.final_cash, 99_950.0, "down-bar cash");
}

/// An order gapped through at the open resolves before any policy applies:
/// the very first observable price of the bar already went through it.
///
/// Bar 2 opens at 111.00, above the 110 target, and its low of 94 is also
/// below the 95 stop. Even under the pessimistic `StopFirst` default the
/// target fills — at the open, 111.00, not at 110.
///   cash = 99 000 + 111.00 × 10 = 100 110, realized = (111 − 100) × 10 = 110.
#[test]
fn golden_a_gapped_level_beats_the_ambiguity_policy() {
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 100.5, 99.5, 100.0),
            (111.0, 112.0, 94.0, 100.0),
            (100.0, 100.5, 99.5, 100.0),
        ],
    );
    let mut strategy = bracket_strategy();
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");

    let exit = &report.fills[1];
    assert_close(exit.fill_price, 111.0, "filled at the gapped open");
    assert_close(exit.realized_pnl, 110.0, "realized");
    assert_close(report.final_cash, 100_110.0, "cash");
}

/// A protective stop gapped through at the open fills at the open (§6.1), so
/// the loss is the real one, not the one the stop price promised.
///
/// Long 10 at 100.00 from bar 1's open; stop at 95 live from bar 2's open;
/// bar 2 opens at 90.00.
///   fill 90.00, realized = (90 − 100) × 10 = −100
///   cash = 99 000 + 90.00 × 10 = 99 900
#[test]
fn golden_gap_through_a_protective_stop_fills_at_the_open() {
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 100.5, 99.5, 100.0),
            (90.0, 91.0, 88.0, 89.0),
            (89.0, 89.5, 88.5, 89.0),
        ],
    );
    let mut strategy = OrderScript::new(vec![
        (0, OrderRequest::market(AAA, OrderSide::Buy, 10.0)),
        (1, OrderRequest::stop(AAA, OrderSide::Sell, 10.0, 95.0)),
    ]);
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");

    let exit = &report.fills[1];
    assert_close(exit.fill_price, 90.0, "gapped stop fills at the open");
    assert_close(exit.realized_pnl, -100.0, "realized");
    assert_close(report.final_cash, 99_900.0, "cash");
    assert_close(report.final_equity, 99_900.0, "equity");
}

/// A limit that never trades. The market ramps 100 → 104 and the buy limit
/// sits at 50, so there is no fill, no fee, and no cash movement — and the
/// order is still resting at the end of the run rather than quietly gone.
#[test]
fn golden_a_limit_order_that_never_fills_changes_nothing() {
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::limit(AAA, OrderSide::Buy, 10.0, 50.0),
    )]);
    let report = run(costed_settings(), &[ramp("aaa", 5)], &mut strategy).expect("runs");

    assert!(report.fills.is_empty(), "no fill");
    assert_close(report.total_commission, 0.0, "no fee");
    assert_close(report.final_cash, START_CAPITAL, "cash untouched");
    assert_close(report.final_equity, START_CAPITAL, "equity untouched");
    assert_eq!(report.pending_orders.len(), 1, "still resting");
    assert!(report.cancellations.is_empty(), "and never cancelled");
}

/// A reversal with costs: close 10 long and open 10 short at the same
/// decision, both filling at the next open.
///
/// Bars: 0 (100, …) decision buy 10; 1 opens 100.00; 2 decision reverse;
/// 3 opens 106.00, closes 106.50.
///
/// Entry  : ask 100.00 + 0.05 + 0.02 slip = 100.07, fee 1.00
///          cash = 100 000 − 1 000.70 − 1.00 = 98 998.30
/// Exit   : bid 106.00 − 0.05 − 0.02 slip = 105.93, fee 1.00
///          realized = (105.93 − 100.07) × 10 = 58.60
///          cash = 98 998.30 + 1 059.30 − 1.00 = 100 056.60
/// Short  : same price 105.93, fee 1.00
///          cash = 100 056.60 + 1 059.30 − 1.00 = 101 114.90
/// Equity : 101 114.90 + (−10 × 106.50) = 100 049.90
#[test]
fn golden_reversal_with_costs() {
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 100.5, 99.5, 100.0),
            (106.0, 107.0, 105.0, 106.5),
        ],
    );
    let mut strategy = ScriptedStrategy::new(vec![
        (0, OrderSide::Buy, 10.0),
        (2, OrderSide::Sell, 10.0),
        (2, OrderSide::Sell, 10.0),
    ]);
    let report = run(costed_settings(), &[stream], &mut strategy).expect("runs");

    assert_eq!(report.fills.len(), 3);
    assert_close(report.fills[0].fill_price, 100.07, "entry fill");
    assert_close(report.fills[0].cash_after, 98_998.30, "cash after entry");

    let close_leg = &report.fills[1];
    assert_close(close_leg.fill_price, 105.93, "closing fill");
    assert_close(close_leg.realized_pnl, 58.60, "realized on the close");
    assert_close(close_leg.position_units_after, 0.0, "flat between legs");
    assert_close(close_leg.cash_after, 100_056.60, "cash after the close");

    let open_leg = &report.fills[2];
    assert_close(open_leg.fill_price, 105.93, "opening fill");
    assert_close(
        open_leg.realized_pnl,
        0.0,
        "an opening leg realizes nothing",
    );
    assert_close(open_leg.position_units_after, -10.0, "now short 10");
    assert_close(open_leg.avg_entry_after, 105.93, "short basis");
    assert_close(open_leg.cash_after, 101_114.90, "cash after the reversal");

    assert_close(report.final_cash, 101_114.90, "final cash");
    assert_close(report.final_equity, 100_049.90, "marked-to-market equity");
    assert_close(report.final_realized_pnl, 58.60, "final realized");
    assert_close(report.total_commission, 3.0, "three fills, three fees");
}

/// The warm-up boundary. With `warmup_bars = 3` a symbol may not submit an
/// order until three of its bars have closed, so decisions on bars 0 and 1 are
/// rejected and reported, and the decision on bar 2 is the first that trades.
#[test]
fn golden_warmup_boundary_rejects_then_admits() {
    let settings = ExecutionSettings {
        warmup_bars: 3,
        ..free_settings()
    };
    let mut strategy = ScriptedStrategy::new(vec![
        (0, OrderSide::Buy, 1.0),
        (1, OrderSide::Buy, 1.0),
        (2, OrderSide::Buy, 1.0),
    ]);
    let report = run(settings, &[ramp("aaa", 5)], &mut strategy).expect("runs");

    assert_eq!(
        report.rejections.len(),
        2,
        "bars 0 and 1 are inside warm-up"
    );
    assert!(matches!(
        report.rejections[0].reason,
        RejectionReason::WarmupIncomplete {
            committed: 1,
            required: 3
        }
    ));
    assert!(matches!(
        report.rejections[1].reason,
        RejectionReason::WarmupIncomplete {
            committed: 2,
            required: 3
        }
    ));

    let fill = only_fill(&report);
    assert_eq!(fill.time_ns, 3 * MINUTE_NS, "the first legal fill");
    assert_close(fill.fill_price, 103.0, "at bar 3's open");
    assert_close(report.final_cash, START_CAPITAL - 103.0, "cash");
}

/// Latency changes which observable price an order can reach. The decision is
/// made at bar 0's close; with no latency the order makes bar 1's open, and
/// with one second of decision→submit delay it does not, because bar 1 was
/// already trading by the time the order existed.
#[test]
fn golden_latency_moves_the_fill_to_the_next_observable_open() {
    let immediate = run(
        free_settings(),
        &[ramp("aaa", 5)],
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 1.0)]),
    )
    .expect("runs");
    let fill = only_fill(&immediate);
    assert_eq!(fill.time_ns, MINUTE_NS);
    assert_close(fill.fill_price, 101.0, "zero-latency fill");

    let delayed = run(
        ExecutionSettings {
            latency: LatencyModel::Fixed {
                decision_to_submit_ns: SECOND_NS,
                submit_to_exchange_ns: 0,
            },
            ..free_settings()
        },
        &[ramp("aaa", 5)],
        &mut ScriptedStrategy::new(vec![(0, OrderSide::Buy, 1.0)]),
    )
    .expect("runs");
    let fill = only_fill(&delayed);
    assert_eq!(fill.time_ns, 2 * MINUTE_NS);
    assert_close(fill.fill_price, 102.0, "one bar later, one point worse");
}

/// Costs must be visible in the result, ordered, and material (§13 M1 gate
/// clause 5). The same 20-bar alternating strategy is run at 0×, 1× and 2×
/// the spread, slippage and commission.
#[test]
fn golden_cost_sensitivity_is_ordered_and_material() {
    let bars: Vec<(f64, f64, f64, f64)> = (0..20)
        .map(|index| {
            let open = 100.0 + f64::from(index % 4);
            (open, open + 1.0, open - 1.0, open + 0.5)
        })
        .collect();
    let script: Vec<(usize, OrderSide, f64)> = (0..18)
        .map(|index| {
            let side = if index % 2 == 0 {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };
            (index, side, 10.0)
        })
        .collect();

    let equity_at = |scale: f64| {
        let settings = ExecutionSettings {
            spread: SpreadModel::Constant {
                price_units: 0.10 * scale,
            },
            slippage: SlippageModel::FixedPriceDistance {
                distance: 0.02 * scale,
            },
            commission: CommissionModel::PerShare {
                amount: 0.01 * scale,
                minimum: 0.0,
            },
            ..free_settings()
        };
        let mut strategy = ScriptedStrategy::new(script.clone());
        run(settings, &[stream_from("aaa", &bars)], &mut strategy)
            .expect("runs")
            .final_equity
    };

    let zero = equity_at(0.0);
    let one = equity_at(1.0);
    let two = equity_at(2.0);

    assert!(
        zero > one,
        "1× costs must be worse than free: {zero} vs {one}"
    );
    assert!(one > two, "2× costs must be worse than 1×: {one} vs {two}");
    // Each round trip pays two half-spreads, two slippages and two fees on 10
    // units; over 18 fills that is far more than a rounding artifact.
    assert!(
        (zero - one) > 10.0 && (one - two) > 10.0,
        "cost steps must be material: {zero} / {one} / {two}"
    );
    // The cost model is linear in the scale, so the two steps are equal.
    assert_close(zero - one, one - two, "cost steps are linear");
}
