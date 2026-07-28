// ── Look-ahead canaries (ADR-135 §6.12, §13 M1 gate clause 3) ──────
//
// Each canary is a strategy that *tries* to cheat. The engine's answer must be
// either "that cannot be written down" — the API offers no way to name a
// future observation — or a deterministic guard error. A canary that quietly
// returned a plausible number would be the failure this suite exists to catch.
//
// Unrepresentable by construction, and therefore asserted by the shape of the
// API rather than by a runtime check:
//
//   * `MarketView` hands out no bar slice, no length, and no iterator, so
//     "scan the whole series" cannot be expressed. The only way in is
//     `bars_ago`, which is `usize` — a negative offset does not exist.
//   * `FormingBar` has no `high`, `low` or `close` field, so a pre-close rule
//     cannot read the values the bar has not printed yet.
//
// Everything else is a guard, and is asserted below.

/// Walks `bars_ago` outward until the guard stops it, recording the deepest
/// answer and the error that ended the walk.
#[derive(Default)]
struct GreedyScanner {
    decisions: usize,
    /// Highest high the strategy could reach at its first decision.
    reachable_high: f64,
    /// The error returned for the first unreachable offset.
    guard: Option<MarketDataError>,
    /// The offset at which the guard tripped.
    guard_at: usize,
}

impl ReferenceStrategy for GreedyScanner {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        _orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        if self.decisions == 0 {
            let market = ctx.market();
            let mut bars_ago = 0usize;
            self.reachable_high = f64::MIN;
            loop {
                match market.high(ctx.symbol(), bars_ago) {
                    Ok(high) => {
                        self.reachable_high = self.reachable_high.max(high);
                        bars_ago += 1;
                    }
                    Err(error) => {
                        self.guard = Some(error);
                        self.guard_at = bars_ago;
                        break;
                    }
                }
            }
        }
        self.decisions += 1;
        Ok(())
    }
}

#[test]
fn canary_scanning_the_whole_series_only_ever_reaches_the_past() {
    // Bar 0 is quiet; bar 3 spikes. A strategy deciding on bar 0's close must
    // not be able to see the spike no matter how it scans.
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 100.5, 99.5, 100.0),
            (100.0, 500.0, 99.5, 100.0),
        ],
    );
    let mut canary = GreedyScanner::default();
    run(free_settings(), &[stream], &mut canary).expect("runs");

    assert_close(canary.reachable_high, 100.5, "deepest reachable high");
    assert_eq!(canary.guard_at, 1, "only bar 0 had closed");
    assert!(matches!(
        canary.guard,
        Some(MarketDataError::FutureData {
            bars_ago: 1,
            available: 1,
            ..
        })
    ));
}

/// Asks for the bar after the one that just closed.
#[derive(Default)]
struct NextBarPeeker {
    tripped: Option<MarketDataError>,
}

impl ReferenceStrategy for NextBarPeeker {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        _orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        // There is no `bars_ahead`. The nearest expressible cheat is to ask for
        // an offset the committed history does not cover, which is exactly what
        // the guard exists to refuse.
        let market = ctx.market();
        let mut committed = 0usize;
        while market.close(ctx.symbol(), committed).is_ok() {
            committed += 1;
        }
        if self.tripped.is_none() {
            self.tripped = market.close(ctx.symbol(), committed).err();
        }
        Ok(())
    }
}

#[test]
fn canary_reading_the_next_bar_trips_the_guard() {
    let mut canary = NextBarPeeker::default();
    run(free_settings(), &[ramp("aaa", 5)], &mut canary).expect("runs");
    assert!(
        matches!(canary.tripped, Some(MarketDataError::FutureData { .. })),
        "the guard did not trip: {:?}",
        canary.tripped
    );
}

/// A pre-close rule that reads everything a forming bar will admit to.
#[derive(Default)]
struct PreCloseReader {
    decisions: usize,
    forming_open: Option<f64>,
    forming_elapsed_ns: Option<i64>,
    latest_committed_close: Option<f64>,
    decision_time_ns: Option<i64>,
}

impl ReferenceStrategy for PreCloseReader {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        _orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        if self.decisions == 1 {
            // `FormingBar` exposes when the bar started and how long it has
            // been running. Its final high, low and close are not fields, so
            // "peek at the close" is not a line of code that compiles.
            let forming = ctx.forming_bar().expect("a pre-close decision has one");
            self.forming_open = Some(forming.open);
            self.forming_elapsed_ns = Some(forming.elapsed_ns);
            self.latest_committed_close = ctx.market().close(ctx.symbol(), 0).ok();
            self.decision_time_ns = Some(ctx.decision_time_ns());
        }
        self.decisions += 1;
        Ok(())
    }
}

#[test]
fn canary_a_pre_close_rule_sees_the_open_and_never_the_final_ohlc() {
    let stream = stream_from(
        "aaa",
        &[
            (100.0, 100.5, 99.5, 100.25),
            (110.0, 190.0, 105.0, 180.0),
            (180.0, 181.0, 179.0, 180.5),
        ],
    );
    let setup = SimulationSetup {
        decision_point: DecisionPoint::PreClose {
            offset_ns: 10 * SECOND_NS,
        },
        ..SimulationSetup::default()
    };
    let mut canary = PreCloseReader::default();
    run_with(free_settings(), setup, &[stream], &mut canary).expect("runs");

    assert_close(
        canary.forming_open.expect("read the forming open"),
        110.0,
        "the forming bar's open is available",
    );
    assert_eq!(
        canary.forming_elapsed_ns,
        Some(MINUTE_NS - 10 * SECOND_NS - 1),
        "and how long it has been forming",
    );
    assert_close(
        canary.latest_committed_close.expect("a committed close"),
        100.25,
        "bars_ago = 0 is still the last *closed* bar, not the forming one",
    );
    assert_eq!(
        canary.decision_time_ns,
        Some(2 * MINUTE_NS - 1 - 10 * SECOND_NS),
        "the decision happens at the configured offset",
    );
}

#[test]
fn canary_a_closed_bar_rule_has_no_forming_bar_at_all() {
    struct FormingProbe {
        seen: Vec<bool>,
    }
    impl ReferenceStrategy for FormingProbe {
        fn on_bar_close(
            &mut self,
            ctx: &DecisionContext<'_>,
            _orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            self.seen.push(ctx.forming_bar().is_some());
            Ok(())
        }
    }
    let mut canary = FormingProbe { seen: Vec::new() };
    run(free_settings(), &[ramp("aaa", 3)], &mut canary).expect("runs");
    assert_eq!(
        canary.seen,
        vec![false, false, false],
        "at a closed-bar decision the next bar has not started"
    );
}

/// Reads a slower symbol from a faster symbol's decision.
#[derive(Default)]
struct HigherTimeframePeeker {
    decisions: usize,
    /// Close of the slow symbol's newest visible bar, at the fast symbol's
    /// second decision.
    visible_slow_close: Option<f64>,
    /// The error from asking for one bar more than the slow symbol has closed.
    guard: Option<MarketDataError>,
}

impl ReferenceStrategy for HigherTimeframePeeker {
    fn on_bar_close(
        &mut self,
        ctx: &DecisionContext<'_>,
        _orders: &mut OrderIntents,
    ) -> Result<(), StrategyError> {
        let market = ctx.market();
        let Some(slow) = market.symbol_id("aaa_slow") else {
            return Ok(());
        };
        if ctx.symbol() == slow {
            return Ok(());
        }
        self.decisions += 1;
        // The fast symbol's 6th bar closes while the slow symbol's second bar
        // is still forming.
        if self.decisions == 6 {
            self.visible_slow_close = market.close(slow, 0).ok();
            let mut committed = 0usize;
            while market.close(slow, committed).is_ok() {
                committed += 1;
            }
            self.guard = market.close(slow, committed).err();
        }
        Ok(())
    }
}

#[test]
fn canary_a_higher_timeframe_bar_is_invisible_until_it_closes() {
    // "aaa_fast": eight one-minute bars. "aaa_slow": two five-minute bars, the
    // first closing at t = 5 min and the second at t = 10 min.
    let fast = SymbolStream {
        symbol: "aaa_fast".to_string(),
        bars: (0..8)
            .map(|index| minute_bar(index, 100.0, 100.5, 99.5, 100.0))
            .collect(),
    };
    let slow = SymbolStream {
        symbol: "aaa_slow".to_string(),
        bars: vec![
            SimBar {
                open_time_ns: 0,
                close_time_ns: 5 * MINUTE_NS - 1,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.75,
                volume: 5_000.0,
            },
            SimBar {
                open_time_ns: 5 * MINUTE_NS,
                close_time_ns: 10 * MINUTE_NS - 1,
                open: 100.75,
                high: 900.0,
                low: 99.0,
                close: 800.0,
                volume: 5_000.0,
            },
        ],
    };

    let mut canary = HigherTimeframePeeker::default();
    run(free_settings(), &[fast, slow], &mut canary).expect("runs");

    assert_close(
        canary
            .visible_slow_close
            .expect("the first slow bar has closed"),
        100.75,
        "only the *closed* higher-timeframe bar is visible",
    );
    assert!(
        matches!(canary.guard, Some(MarketDataError::FutureData { .. })),
        "reaching past it must trip the guard: {:?}",
        canary.guard
    );
}

#[test]
fn canary_a_future_dated_indicator_input_is_a_guard_trip_not_a_number() {
    // An indicator dated after the decision is, at this layer, exactly a read
    // of a bar that has not closed. The strategy below tries to build a
    // "future-dated moving average" from the deepest offsets it can name and
    // must be stopped at the boundary rather than handed a value.
    #[derive(Default)]
    struct FutureAverage {
        error: Option<MarketDataError>,
        average_of_the_past: Option<f64>,
    }
    impl ReferenceStrategy for FutureAverage {
        fn on_bar_close(
            &mut self,
            ctx: &DecisionContext<'_>,
            _orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            if self.error.is_some() {
                return Ok(());
            }
            let market = ctx.market();
            // Ask for a three-bar average at the first decision, when only one
            // bar has closed: the two missing samples are the future.
            let mut sum = 0.0;
            for bars_ago in 0..3 {
                match market.close(ctx.symbol(), bars_ago) {
                    Ok(close) => sum += close,
                    Err(error) => {
                        self.error = Some(error);
                        return Ok(());
                    }
                }
            }
            self.average_of_the_past = Some(sum / 3.0);
            Ok(())
        }
    }

    let mut canary = FutureAverage::default();
    run(free_settings(), &[ramp("aaa", 4)], &mut canary).expect("runs");
    assert!(
        matches!(
            canary.error,
            Some(MarketDataError::FutureData { bars_ago: 1, .. })
        ),
        "expected a guard trip, got {:?}",
        canary.error
    );
    assert_eq!(
        canary.average_of_the_past, None,
        "the cheating average must never have been produced"
    );
}

/// The position feedback added for protective management (§10.3) is new state
/// handed to strategies, so it needs its own canary: a high-water mark that
/// included the bar being decided *on* at a pre-close decision would leak that
/// bar's unfinished range.
///
/// At a pre-close decision the forming bar has not closed, so
/// `favorable_extreme` may only reflect bars that have. The ramp's bar `i` has
/// high `101 + i`, so a decision inside bar `i` must never see beyond
/// `100 + i`.
#[test]
fn the_position_high_water_mark_never_includes_the_forming_bar() {
    #[derive(Default)]
    struct ExtremeProbe {
        decisions: usize,
        /// (decision index, forming bar high water mark) once long.
        observed: Vec<(usize, f64)>,
    }

    impl ReferenceStrategy for ExtremeProbe {
        fn on_bar_close(
            &mut self,
            ctx: &DecisionContext<'_>,
            orders: &mut OrderIntents,
        ) -> Result<(), StrategyError> {
            let now = self.decisions;
            self.decisions += 1;
            if now == 0 {
                orders.market(ctx.symbol(), OrderSide::Buy, 1.0)?;
                return Ok(());
            }
            let position = ctx.own_position();
            if !position.is_flat() {
                self.observed.push((now, position.favorable_extreme));
            }
            Ok(())
        }
    }

    let mut canary = ExtremeProbe::default();
    let setup = SimulationSetup {
        decision_point: DecisionPoint::PreClose {
            offset_ns: HALF_SECOND_NS,
        },
        ..SimulationSetup::default()
    };
    run_with(free_settings(), setup, &[ramp("aaa", 6)], &mut canary).expect("runs");

    assert!(
        !canary.observed.is_empty(),
        "the probe must actually have held a position"
    );
    for (decision, extreme) in canary.observed {
        // A pre-close decision inside bar `decision` may only have seen bars
        // 0..decision-1, whose highest high is 100 + decision.
        let highest_committed = 100.0 + decision as f64;
        assert!(
            extreme <= highest_committed + 1e-9,
            "decision {decision} saw {extreme}, beyond the committed high \
             {highest_committed}"
        );
    }
}
