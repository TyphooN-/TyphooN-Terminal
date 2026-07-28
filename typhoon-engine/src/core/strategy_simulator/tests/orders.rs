// ── Order types and lifecycle (§6.5) ───────────────────────────────

/// The one symbol every single-stream scenario decides on.
const AAA: SymbolId = SymbolId(0);

#[test]
fn a_touched_buy_limit_fills_at_its_limit_price() {
    // Bar 1 trades down through 99.50 and back up; the limit is touched, not
    // gapped, so the fill is exactly the limit.
    let stream = stream_from(
        "aaa",
        &[(100.0, 100.5, 99.8, 100.0), (100.0, 101.0, 99.0, 100.5)],
    );
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::limit(AAA, OrderSide::Buy, 10.0, 99.5),
    )]);
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");

    let fill = only_fill(&report);
    assert_close(fill.fill_price, 99.5, "fill price");
    assert_eq!(
        fill.time_ns,
        2 * MINUTE_NS - 1,
        "a non-gap range touch is not knowable before the bar closes"
    );
}

#[test]
fn a_gapped_buy_limit_fills_at_the_better_open() {
    // Bar 1 opens below the limit: the first observable price is already
    // through it, so the fill is the open, not the limit.
    let stream = stream_from(
        "aaa",
        &[(100.0, 100.5, 99.8, 100.0), (98.0, 99.0, 97.0, 98.5)],
    );
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::limit(AAA, OrderSide::Buy, 10.0, 99.5),
    )]);
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");

    assert_close(only_fill(&report).fill_price, 98.0, "gapped limit fill");
}

#[test]
fn a_touched_buy_stop_fills_at_the_stop_and_a_gapped_one_at_the_open() {
    let touched = stream_from(
        "aaa",
        &[(100.0, 100.5, 99.8, 100.0), (100.0, 103.0, 99.5, 102.0)],
    );
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::stop(AAA, OrderSide::Buy, 10.0, 102.0),
    )]);
    let report = run(intrabar_settings(), &[touched], &mut strategy).expect("runs");
    assert_close(only_fill(&report).fill_price, 102.0, "touched stop");

    let gapped = stream_from(
        "aaa",
        &[(100.0, 100.5, 99.8, 100.0), (105.0, 106.0, 104.0, 105.5)],
    );
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::stop(AAA, OrderSide::Buy, 10.0, 102.0),
    )]);
    let report = run(intrabar_settings(), &[gapped], &mut strategy).expect("runs");
    assert_close(only_fill(&report).fill_price, 105.0, "gapped stop");
}

#[test]
fn a_stop_limit_fills_at_the_trigger_when_marketable_and_rests_otherwise() {
    // Trigger 102, limit 103: at the trigger the price is inside the limit, so
    // it executes at 102 immediately.
    let stream = stream_from(
        "aaa",
        &[(100.0, 100.5, 99.8, 100.0), (100.0, 103.0, 99.5, 102.5)],
    );
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::stop_limit(AAA, OrderSide::Buy, 10.0, 102.0, 103.0),
    )]);
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");
    assert_close(
        only_fill(&report).fill_price,
        102.0,
        "marketable stop-limit",
    );

    // Trigger 102, limit 101.5: the trigger price is worse than the limit, so
    // nothing fills on the trigger bar — the path inside it is unknown — and
    // the order rests as a plain limit. Bar 2 never trades back to 101.5.
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.8, 100.0),
            (100.0, 103.0, 99.5, 102.5),
            (102.5, 104.0, 102.0, 103.0),
        ],
    );
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::stop_limit(AAA, OrderSide::Buy, 10.0, 102.0, 101.5),
    )]);
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");
    assert!(report.fills.is_empty(), "an unmarketable stop-limit rests");
    assert_eq!(report.pending_orders.len(), 1, "and stays live");
}

#[test]
fn a_market_on_close_order_never_fills_at_the_close_that_decided_it() {
    let stream = stream_from(
        "aaa",
        &[(100.0, 100.5, 99.8, 100.0), (101.0, 102.0, 100.5, 101.5)],
    );
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::market_on_close(AAA, OrderSide::Buy, 10.0),
    )]);
    let report = run(free_settings(), &[stream], &mut strategy).expect("runs");

    let fill = only_fill(&report);
    assert_close(fill.fill_price, 101.5, "fills at the next bar's close");
    assert_eq!(fill.time_ns, 2 * MINUTE_NS - 1, "at that bar's close stamp");
}

#[test]
fn an_immediate_or_cancel_order_expires_on_its_first_eligible_bar() {
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.8, 100.0),
            (100.0, 101.0, 99.9, 100.5),
            (100.5, 101.0, 99.0, 100.0),
        ],
    );
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::limit(AAA, OrderSide::Buy, 10.0, 99.5).with_tif(TimeInForce::Ioc),
    )]);
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");

    assert!(report.fills.is_empty(), "bar 1 never reaches 99.5");
    assert!(report.pending_orders.is_empty(), "and IOC does not rest");
    assert_eq!(report.cancellations.len(), 1);
    assert_eq!(report.cancellations[0].reason, CancelReason::Expired);
    assert_eq!(
        report.cancellations[0].time_ns,
        2 * MINUTE_NS - 1,
        "expires at the close of the bar it could not fill in"
    );
}

#[test]
fn a_day_order_expires_at_the_end_of_its_utc_day() {
    // 1,440 one-minute bars is exactly one UTC day, so bar 1,440 opens on the
    // next day and the order must already be gone.
    let bars = (0..1_442)
        .map(|index| minute_bar(index, 100.0, 100.5, 99.5, 100.0))
        .collect();
    let stream = SymbolStream {
        symbol: "aaa".to_string(),
        bars,
    };
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::limit(AAA, OrderSide::Buy, 10.0, 50.0).with_tif(TimeInForce::Day),
    )]);
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");

    assert!(report.fills.is_empty());
    assert!(report.pending_orders.is_empty());
    assert_eq!(report.cancellations.len(), 1);
    assert_eq!(report.cancellations[0].reason, CancelReason::Expired);
    assert_eq!(
        report.cancellations[0].time_ns,
        1_440 * MINUTE_NS,
        "expires at the first instant of the next UTC day"
    );
}

#[test]
fn a_good_til_date_order_expires_at_its_stamp() {
    let stream = ramp("aaa", 5);
    let expire_time_ns = 3 * MINUTE_NS + 15 * SECOND_NS;
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::limit(AAA, OrderSide::Buy, 10.0, 50.0)
            .with_tif(TimeInForce::Gtd { expire_time_ns }),
    )]);
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");

    assert_eq!(report.cancellations.len(), 1);
    assert_eq!(report.cancellations[0].time_ns, expire_time_ns);
    assert!(report.pending_orders.is_empty());
}

#[test]
fn a_cancel_request_removes_a_resting_order() {
    struct CancelAfter {
        decisions: usize,
        id: Option<ClientOrderId>,
    }
    impl ReferenceStrategy for CancelAfter {
        fn on_bar_close(
            &mut self,
            ctx: &DecisionContext<'_>,
            orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            let now = self.decisions;
            self.decisions += 1;
            match now {
                0 => {
                    self.id = Some(orders.submit(OrderRequest::limit(
                        ctx.symbol(),
                        OrderSide::Buy,
                        10.0,
                        50.0,
                    ))?);
                }
                1 => {
                    let id = self.id.take().expect("submitted on decision 0");
                    orders.cancel(id)?;
                }
                _ => {}
            }
            Ok(())
        }
    }

    let mut strategy = CancelAfter {
        decisions: 0,
        id: None,
    };
    let report = run(intrabar_settings(), &[ramp("aaa", 4)], &mut strategy).expect("runs");

    assert!(report.pending_orders.is_empty(), "the order was cancelled");
    assert_eq!(report.cancellations.len(), 1);
    assert_eq!(report.cancellations[0].reason, CancelReason::Requested);
    assert_eq!(
        report.cancellations[0].time_ns,
        2 * MINUTE_NS,
        "the request lands one causal nanosecond after the decision that made it"
    );
}

#[test]
fn a_modify_request_repriced_a_resting_limit_into_a_fill() {
    struct ModifyAfter {
        decisions: usize,
        id: Option<ClientOrderId>,
    }
    impl ReferenceStrategy for ModifyAfter {
        fn on_bar_close(
            &mut self,
            ctx: &DecisionContext<'_>,
            orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            let now = self.decisions;
            self.decisions += 1;
            match now {
                0 => {
                    self.id = Some(orders.submit(OrderRequest::limit(
                        ctx.symbol(),
                        OrderSide::Buy,
                        10.0,
                        50.0,
                    ))?);
                }
                1 => {
                    let id = self.id.take().expect("submitted on decision 0");
                    orders.modify(id, ModifyRequest::limit_price(101.0))?;
                }
                _ => {}
            }
            Ok(())
        }
    }

    // Bar 2 trades down to 101 exactly once, and only the repriced order can
    // reach it.
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.8, 100.0),
            (102.0, 102.5, 101.8, 102.0),
            (102.0, 103.0, 100.5, 102.5),
        ],
    );
    let mut strategy = ModifyAfter {
        decisions: 0,
        id: None,
    };
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");

    assert_close(only_fill(&report).fill_price, 101.0, "repriced fill");
}

#[test]
fn cancelling_or_modifying_an_unknown_order_is_reported_not_ignored() {
    struct CancelGhost;
    impl ReferenceStrategy for CancelGhost {
        fn on_bar_close(
            &mut self,
            _ctx: &DecisionContext<'_>,
            orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            orders.cancel(ClientOrderId(9_999))?;
            Ok(())
        }
    }
    let report = run(free_settings(), &[ramp("aaa", 2)], &mut CancelGhost).expect("runs");
    assert_eq!(report.rejections.len(), 2, "one per decision");
    assert!(matches!(
        report.rejections[0].reason,
        RejectionReason::UnknownOrder
    ));
}

#[test]
fn an_oco_sibling_is_cancelled_when_its_partner_fills() {
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.8, 100.0),
            (100.0, 112.0, 94.0, 101.0),
            (101.0, 101.5, 100.5, 101.0),
        ],
    );
    let mut strategy = OrderScript::new(vec![
        (
            0,
            OrderRequest::stop(AAA, OrderSide::Sell, 10.0, 95.0).with_oco(1),
        ),
        (
            0,
            OrderRequest::limit(AAA, OrderSide::Sell, 10.0, 110.0).with_oco(1),
        ),
    ]);
    let report = run(intrabar_settings(), &[stream], &mut strategy).expect("runs");

    assert_eq!(report.fills.len(), 1, "only one leg of the bracket fills");
    assert_eq!(report.cancellations.len(), 1);
    assert_eq!(report.cancellations[0].reason, CancelReason::OcoSibling);
    assert!(report.pending_orders.is_empty());
}

#[test]
fn a_reduce_only_order_is_rejected_when_it_would_open_or_grow_a_position() {
    let mut flat = OrderScript::new(vec![(
        0,
        OrderRequest::market(AAA, OrderSide::Sell, 10.0).reduce_only(),
    )]);
    let report = run(free_settings(), &[ramp("aaa", 3)], &mut flat).expect("runs");
    assert!(report.fills.is_empty());
    assert_eq!(report.rejections.len(), 1);
    assert!(matches!(
        report.rejections[0].reason,
        RejectionReason::ReduceOnlyWouldNotReduce
    ));

    // Long 10, then a reduce-only sell of 25 would flip the position.
    let mut oversized = OrderScript::new(vec![
        (0, OrderRequest::market(AAA, OrderSide::Buy, 10.0)),
        (
            2,
            OrderRequest::market(AAA, OrderSide::Sell, 25.0).reduce_only(),
        ),
    ]);
    let report = run(free_settings(), &[ramp("aaa", 5)], &mut oversized).expect("runs");
    assert_eq!(report.fills.len(), 1, "only the entry fills");
    assert_eq!(report.rejections.len(), 1);
    assert!(matches!(
        report.rejections[0].reason,
        RejectionReason::ReduceOnlyExceedsPosition { .. }
    ));
}

#[test]
fn a_cash_account_refuses_a_short_and_an_unaffordable_buy() {
    let settings = ExecutionSettings {
        margin: MarginPolicy::CashOnly,
        initial_capital: 1_000.0,
        ..free_settings()
    };

    let mut short = OrderScript::new(vec![(0, OrderRequest::market(AAA, OrderSide::Sell, 1.0))]);
    let report = run(settings.clone(), &[ramp("aaa", 3)], &mut short).expect("runs");
    assert!(report.fills.is_empty());
    assert!(matches!(
        report.rejections[0].reason,
        RejectionReason::ShortNotPermitted
    ));

    let mut greedy = OrderScript::new(vec![(0, OrderRequest::market(AAA, OrderSide::Buy, 100.0))]);
    let report = run(settings, &[ramp("aaa", 3)], &mut greedy).expect("runs");
    assert!(report.fills.is_empty());
    assert!(matches!(
        report.rejections[0].reason,
        RejectionReason::InsufficientBuyingPower { .. }
    ));
}

#[test]
fn an_off_tick_price_is_rejected_at_submission() {
    let settings = ExecutionSettings {
        price_tick: Some(0.01),
        ..free_settings()
    };
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::limit(AAA, OrderSide::Buy, 10.0, 99.5051),
    )]);
    let report = run(settings, &[ramp("aaa", 3)], &mut strategy).expect("runs");

    assert!(report.fills.is_empty());
    assert_eq!(report.rejections.len(), 1);
    assert!(matches!(
        report.rejections[0].reason,
        RejectionReason::PriceOffTick { .. }
    ));
    assert_eq!(
        report.rejections[0].time_ns,
        MINUTE_NS - 1,
        "rejected where it was submitted, not silently later"
    );
}

#[test]
fn a_non_finite_or_non_positive_price_is_refused_before_it_reaches_the_book() {
    for price in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        let mut strategy = OrderScript::new(vec![(
            0,
            OrderRequest::limit(AAA, OrderSide::Buy, 1.0, price),
        )]);
        let result = run(free_settings(), &[ramp("aaa", 3)], &mut strategy);
        assert!(
            matches!(
                result,
                Err(SimulationError::Strategy {
                    error: StrategyError::InvalidPrice { .. },
                    ..
                })
            ),
            "price {price} was accepted"
        );
    }
}

#[test]
fn bar_close_fidelity_never_resolves_a_trigger_inside_a_bar() {
    // The bar's low reaches 94 but neither its open nor its close does. At
    // bar-close fidelity the stop must not fire; at OHLC fidelity it must.
    let rows = [
        (100.0, 100.5, 99.8, 100.0),
        (100.0, 101.0, 94.0, 100.0),
        (100.0, 100.5, 99.8, 100.0),
    ];
    let request = OrderRequest::stop(AAA, OrderSide::Sell, 10.0, 95.0);

    let mut coarse = OrderScript::new(vec![(0, request.clone())]);
    let report = run(free_settings(), &[stream_from("aaa", &rows)], &mut coarse).expect("runs");
    assert!(report.fills.is_empty(), "bar-close fidelity sees no path");

    let mut fine = OrderScript::new(vec![(0, request)]);
    let report = run(intrabar_settings(), &[stream_from("aaa", &rows)], &mut fine).expect("runs");
    assert_close(only_fill(&report).fill_price, 95.0, "intrabar stop");
}

#[test]
fn an_order_is_not_eligible_for_a_bar_that_was_already_open_when_it_activated() {
    // 30 s of submit latency lands inside bar 1, so bar 1's range is not a
    // path this order could have traded — it waits for bar 2's open.
    let settings = ExecutionSettings {
        latency: LatencyModel::Fixed {
            decision_to_submit_ns: 30 * SECOND_NS,
            submit_to_exchange_ns: 0,
        },
        ..intrabar_settings()
    };
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.8, 100.0),
            (100.0, 101.0, 94.0, 100.0),
            (100.0, 100.5, 99.9, 100.0),
        ],
    );
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::stop(AAA, OrderSide::Sell, 10.0, 95.0),
    )]);
    let report = run(settings, &[stream], &mut strategy).expect("runs");

    assert!(
        report.fills.is_empty(),
        "bar 1's low happened while the order was still in flight"
    );
    assert_eq!(report.pending_orders.len(), 1);
}

#[test]
fn submit_and_activate_events_carry_the_configured_latency() {
    let settings = ExecutionSettings {
        latency: LatencyModel::Fixed {
            decision_to_submit_ns: HALF_SECOND_NS,
            submit_to_exchange_ns: HALF_SECOND_NS,
        },
        ..free_settings()
    };
    let mut strategy = ScriptedStrategy::new(vec![(0, OrderSide::Buy, 1.0)]);
    let report = run(settings, &[ramp("aaa", 4)], &mut strategy).expect("runs");

    let decision = MINUTE_NS - 1;
    let submit = report
        .events
        .iter()
        .find(|event| event.kind == SimEventKind::OrderSubmit)
        .expect("an order was submitted");
    let activate = report
        .events
        .iter()
        .find(|event| event.kind == SimEventKind::OrderActivate)
        .expect("and activated");
    assert_eq!(submit.time_ns, decision + HALF_SECOND_NS);
    assert_eq!(activate.time_ns, decision + 2 * HALF_SECOND_NS);
}

#[test]
fn a_bar_delayed_submission_waits_for_that_many_bar_opens() {
    let setup = SimulationSetup {
        submit_delay_bars: 2,
        ..SimulationSetup::default()
    };
    let mut strategy = ScriptedStrategy::new(vec![(0, OrderSide::Buy, 1.0)]);
    let report = run_with(free_settings(), setup, &[ramp("aaa", 6)], &mut strategy).expect("runs");

    let fill = only_fill(&report);
    // Decision on bar 0; submitted at bar 2's open; first eligible open is
    // bar 3's, whose price is 103.
    assert_eq!(fill.time_ns, 3 * MINUTE_NS);
    assert_close(fill.fill_price, 103.0, "delayed fill price");
}

#[test]
fn a_bar_delayed_submission_that_runs_off_the_stream_is_reported() {
    let setup = SimulationSetup {
        submit_delay_bars: 4,
        ..SimulationSetup::default()
    };
    let mut strategy = ScriptedStrategy::new(vec![(2, OrderSide::Buy, 1.0)]);
    let report = run_with(free_settings(), setup, &[ramp("aaa", 4)], &mut strategy).expect("runs");

    assert!(report.fills.is_empty());
    assert_eq!(report.rejections.len(), 1);
    assert!(matches!(
        report.rejections[0].reason,
        RejectionReason::SubmitWindowUnavailable { .. }
    ));
}

#[test]
fn a_next_bar_open_decision_sees_the_open_and_still_cannot_trade_it() {
    struct OpenReader {
        seen: Vec<f64>,
    }
    impl ReferenceStrategy for OpenReader {
        fn on_bar_close(
            &mut self,
            ctx: &DecisionContext<'_>,
            orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            if let Ok(open) = ctx.market().opening_price(ctx.symbol()) {
                self.seen.push(open);
            }
            if self.seen.len() == 1 {
                orders.market(ctx.symbol(), OrderSide::Buy, 1.0)?;
            }
            Ok(())
        }
    }

    let setup = SimulationSetup {
        decision_point: DecisionPoint::NextBarOpen,
        ..SimulationSetup::default()
    };
    let mut strategy = OpenReader { seen: Vec::new() };
    let report = run_with(free_settings(), setup, &[ramp("aaa", 4)], &mut strategy).expect("runs");

    assert_eq!(strategy.seen.first().copied(), Some(100.0));
    let fill = only_fill(&report);
    assert_eq!(fill.time_ns, MINUTE_NS, "the order waits for the next open");
    assert_close(fill.fill_price, 101.0, "and pays that open");
}

#[test]
fn rejections_are_recorded_with_the_order_that_caused_them() {
    let settings = ExecutionSettings {
        warmup_bars: 2,
        ..free_settings()
    };
    let mut strategy = ScriptedStrategy::new(vec![(0, OrderSide::Buy, 1.0)]);
    let report = run(settings, &[ramp("aaa", 4)], &mut strategy).expect("runs");

    assert_eq!(report.rejections.len(), 1);
    let rejection = &report.rejections[0];
    assert_eq!(rejection.symbol, AAA);
    assert_eq!(rejection.client_order_id, ClientOrderId(0));
    assert!(matches!(
        rejection.reason,
        RejectionReason::WarmupIncomplete {
            committed: 1,
            required: 2
        }
    ));
    assert!(
        report
            .events
            .iter()
            .any(|event| event.kind == SimEventKind::OrderReject),
        "a rejection is an event, not just a footnote"
    );
}

#[test]
fn bar_ohlc_range_touches_do_not_time_travel_before_staggered_events() {
    let aaa = SymbolStream {
        symbol: "aaa".into(),
        bars: vec![
            SimBar {
                open_time_ns: -MINUTE_NS,
                close_time_ns: -1,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 1_000.0,
            },
            minute_bar(0, 100.0, 105.0, 99.0, 104.0),
        ],
    };
    let bbb = SymbolStream {
        symbol: "bbb".into(),
        bars: vec![SimBar {
            open_time_ns: -MINUTE_NS / 2,
            close_time_ns: MINUTE_NS / 2 - 1,
            open: 200.0,
            high: 201.0,
            low: 199.0,
            close: 200.0,
            volume: 1_000.0,
        }],
    };
    let mut strategy = OrderScript::new(vec![(
        0,
        OrderRequest::stop(AAA, OrderSide::Buy, 1.0, 103.0),
    )]);
    let report = run(intrabar_settings(), &[aaa, bbb], &mut strategy).expect("runs");

    let fill = only_fill(&report);
    assert_eq!(fill.time_ns, MINUTE_NS - 1);
    let intervening = report
        .events
        .iter()
        .find(|event| event.kind == SimEventKind::Decision && event.time_ns == MINUTE_NS / 2 - 1)
        .expect("the staggered symbol decides while aaa is still forming");
    assert!(intervening.sequence < fill.sequence);
}

#[test]
fn a_triggered_resting_stop_limit_fills_later_without_retrigger_at_both_fidelities() {
    for settings in [free_settings(), intrabar_settings()] {
        let stream = stream_from(
            "aaa",
            &[
                (100.0, 100.5, 99.8, 100.0),
                (100.0, 103.0, 99.5, 102.0),
                (101.5, 101.9, 101.0, 101.7),
            ],
        );
        let mut strategy = OrderScript::new(vec![(
            0,
            OrderRequest::stop_limit(AAA, OrderSide::Buy, 1.0, 102.0, 101.5),
        )]);
        let report = run(settings, &[stream], &mut strategy).expect("runs");

        assert_close(only_fill(&report).fill_price, 101.5, "resting limit fill");
        assert_eq!(
            report
                .events
                .iter()
                .filter(|event| event.kind == SimEventKind::StopTriggered)
                .count(),
            1,
            "the stop transition happens once"
        );
    }
}

#[test]
fn a_pre_close_moc_fills_only_when_active_by_that_close() {
    let stream = stream_from(
        "aaa",
        &[(100.0, 101.0, 99.0, 100.5), (101.0, 102.0, 100.0, 101.5)],
    );
    let setup = SimulationSetup {
        decision_point: DecisionPoint::PreClose {
            offset_ns: 10 * SECOND_NS,
        },
        ..SimulationSetup::default()
    };

    for (latency_ns, expected_time) in [
        (0, MINUTE_NS - 1),
        (10 * SECOND_NS - 1, MINUTE_NS - 1),
        (10 * SECOND_NS, 2 * MINUTE_NS - 1),
    ] {
        let settings = ExecutionSettings {
            latency: LatencyModel::Fixed {
                decision_to_submit_ns: latency_ns,
                submit_to_exchange_ns: 0,
            },
            ..free_settings()
        };
        let mut strategy = OrderScript::new(vec![(
            0,
            OrderRequest::market_on_close(AAA, OrderSide::Buy, 1.0),
        )]);
        let report =
            run_with(settings, setup.clone(), &[stream.clone()], &mut strategy).expect("runs");
        assert_eq!(
            only_fill(&report).time_ns,
            expected_time,
            "latency {latency_ns}"
        );
    }
}

#[test]
fn a_rejected_modify_leaves_every_order_field_unchanged() {
    struct InvalidModify {
        decisions: usize,
        id: Option<ClientOrderId>,
        off_tick: bool,
    }
    impl ReferenceStrategy for InvalidModify {
        fn on_bar_close(
            &mut self,
            ctx: &DecisionContext<'_>,
            orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            match self.decisions {
                0 => {
                    self.id = Some(orders.submit(OrderRequest::limit(
                        ctx.symbol(),
                        OrderSide::Buy,
                        2.0,
                        90.0,
                    ))?);
                }
                1 => {
                    let change = if self.off_tick {
                        ModifyRequest {
                            quantity: Some(7.0),
                            limit_price: Some(90.005),
                            stop_price: None,
                        }
                    } else {
                        ModifyRequest {
                            quantity: Some(7.0),
                            limit_price: None,
                            stop_price: Some(80.0),
                        }
                    };
                    orders.modify(self.id.expect("submitted"), change)?;
                }
                _ => {}
            }
            self.decisions += 1;
            Ok(())
        }
    }

    for off_tick in [false, true] {
        let settings = ExecutionSettings {
            price_tick: Some(0.01),
            ..free_settings()
        };
        let mut strategy = InvalidModify {
            decisions: 0,
            id: None,
            off_tick,
        };
        let report = run(settings, &[ramp("aaa", 3)], &mut strategy).expect("runs");
        assert_eq!(report.rejections.len(), 1);
        let pending = report.pending_orders.first().expect("original order rests");
        assert_eq!(pending.quantity, 2.0, "quantity is committed atomically");
        assert_eq!(pending.kind, OrderKind::Limit { limit_price: 90.0 });
    }
}

/// A reversal is a new position, so the anchors protective management hangs off
/// must not be inherited from the position that was reversed.
///
/// Ramp bars: bar `i` opens at `100 + i`, high `101 + i`, low `99 + i`.
/// Decision 0 buys 1; it fills at bar 1's open (101.00). Decision 2 sells 3,
/// flipping to short 2 at bar 3's open (103.00). The short's entry time and
/// high-water mark must both be stamped at that flip, not carried over.
#[test]
fn a_reversal_restamps_the_position_entry_time_and_high_water_mark() {
    #[derive(Default)]
    struct FlipProbe {
        decisions: usize,
        long_opened_at: Option<i64>,
        short_opened_at: Option<i64>,
        short_extreme: Option<f64>,
    }

    impl ReferenceStrategy for FlipProbe {
        fn on_bar_close(
            &mut self,
            ctx: &DecisionContext<'_>,
            orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            let now = self.decisions;
            self.decisions += 1;
            let position = ctx.own_position();
            if position.is_long() && self.long_opened_at.is_none() {
                self.long_opened_at = Some(position.opened_time_ns);
            }
            if position.is_short() {
                self.short_opened_at.get_or_insert(position.opened_time_ns);
                self.short_extreme = Some(position.favorable_extreme);
            }
            match now {
                0 => {
                    orders.market(ctx.symbol(), OrderSide::Buy, 1.0)?;
                }
                2 => {
                    orders.market(ctx.symbol(), OrderSide::Sell, 3.0)?;
                }
                _ => {}
            }
            Ok(())
        }
    }

    let mut probe = FlipProbe::default();
    run(free_settings(), &[ramp("aaa", 6)], &mut probe).expect("runs");

    assert_eq!(
        probe.long_opened_at,
        Some(MINUTE_NS),
        "the long was opened at bar 1's open"
    );
    assert_eq!(
        probe.short_opened_at,
        Some(3 * MINUTE_NS),
        "the flip restamps the entry time rather than keeping the long's"
    );
    // The short is stamped at 103.00 and then tracks lows. Bar 3's low is
    // 102.00, so by the next decision the extreme has moved down, never up.
    let extreme = probe.short_extreme.expect("short observed");
    assert!(
        extreme <= 103.0 + 1e-9,
        "a short's high-water mark moves down from its entry, got {extreme}"
    );
}
