//! Versioned, deterministic metrics computed from the M1 simulation ledger.
//!
//! This is deliberately a consumer of [`SimulationReport`], not another
//! execution engine. Metric values never use NaN, infinity, or magic sentinels;
//! undefined statistics carry a typed reason.

use crate::core::strategy_simulator::{OrderSide, SimulationReport, SymbolId, SymbolStream};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub const METRICS_SCHEMA_VERSION: &str = "strategy-metrics/v1";
const DAY_NS: i64 = 86_400_000_000_000;
/// Trading days per year — the annualization factor for daily-return ratios.
const YEAR_DAYS: f64 = 252.0;
/// Mean Gregorian year in nanoseconds. Wall-clock elapsed time is calendar time,
/// so compounding and per-year rates use 365.2425 days, not the 252 trading days
/// that annualize a *daily-return* series.
const YEAR_NS: i64 = 31_556_952_000_000_000;
const MAX_METRIC_INPUT_ITEMS: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UndefinedReason {
    NoTrades,
    NoWinningTrades,
    NoLosingTrades,
    ZeroDenominator,
    ZeroVariance,
    MissingInitialRisk,
    InsufficientObservations,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum MetricValue {
    Defined { value: f64 },
    Undefined { reason: UndefinedReason },
}

impl MetricValue {
    pub fn defined(value: f64) -> Self {
        if value.is_finite() {
            // Negative zero has no metric meaning and must not create a second
            // serialized/hashed representation of zero.
            Self::Defined {
                value: if value == 0.0 { 0.0 } else { value },
            }
        } else {
            Self::Undefined {
                reason: UndefinedReason::ArithmeticOverflow,
            }
        }
    }

    pub const fn undefined(reason: UndefinedReason) -> Self {
        Self::Undefined { reason }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MetricDefinition {
    pub id: &'static str,
    pub formula: &'static str,
    pub units: &'static str,
    pub periodicity: &'static str,
    pub annualization: &'static str,
    pub degenerate_case: &'static str,
}

macro_rules! metric {
    ($id:literal, $formula:literal, $units:literal, $period:literal, $annual:literal, $degenerate:literal) => {
        MetricDefinition {
            id: $id,
            formula: $formula,
            units: $units,
            periodicity: $period,
            annualization: $annual,
            degenerate_case: $degenerate,
        }
    };
}

static REGISTRY: &[MetricDefinition] = &[
    metric!(
        "net_profit",
        "sum(closed_trade.net_pnl)",
        "account_currency",
        "run",
        "none",
        "0 when no closed trades"
    ),
    metric!(
        "gross_profit",
        "sum(max(closed_trade.net_pnl, 0))",
        "account_currency",
        "run",
        "none",
        "0 when no winning trades"
    ),
    metric!(
        "gross_loss",
        "sum(min(closed_trade.net_pnl, 0))",
        "account_currency",
        "run",
        "none",
        "0 when no losing trades"
    ),
    metric!(
        "total_return",
        "(final_equity - initial_equity) / initial_equity",
        "ratio",
        "run",
        "none",
        "undefined when initial equity is zero"
    ),
    metric!(
        "average_trade",
        "mean(closed_trade.net_pnl)",
        "account_currency/trade",
        "trade",
        "none",
        "undefined when no closed trades"
    ),
    metric!(
        "expectancy",
        "win_rate*mean(win) + loss_rate*mean(loss)",
        "account_currency/trade",
        "trade",
        "none",
        "undefined when no closed trades"
    ),
    metric!(
        "profit_factor",
        "gross_profit / abs(gross_loss)",
        "ratio",
        "run",
        "none",
        "undefined when there are no losing trades"
    ),
    metric!(
        "payoff_ratio",
        "mean(win) / abs(mean(loss))",
        "ratio",
        "trade",
        "none",
        "undefined without both winning and losing trades"
    ),
    metric!(
        "max_drawdown_absolute",
        "max(running_peak_equity - equity)",
        "account_currency",
        "mark_to_market",
        "none",
        "0 for an empty/flat curve"
    ),
    metric!(
        "max_drawdown_percent",
        "max((running_peak_equity - equity) / running_peak_equity)",
        "ratio",
        "mark_to_market",
        "none",
        "undefined when a relevant peak is non-positive"
    ),
    metric!(
        "max_drawdown_duration",
        "max(time below prior equity peak)",
        "nanoseconds",
        "mark_to_market",
        "none",
        "0 for an empty/flat curve"
    ),
    metric!(
        "ulcer_index",
        "sqrt(mean(drawdown_percent^2))",
        "ratio",
        "mark_to_market",
        "none",
        "undefined when no positive equity peak exists"
    ),
    metric!(
        "longest_stagnation",
        "max(time at or below a prior equity peak)",
        "nanoseconds",
        "mark_to_market",
        "none",
        "0 for an empty/strictly rising curve"
    ),
    metric!(
        "sharpe_ratio",
        "mean(daily_return) / sample_stddev(daily_return) * sqrt(252), risk_free=0",
        "ratio",
        "daily",
        "sqrt(252)",
        "undefined with fewer than 2 returns or zero variance"
    ),
    metric!(
        "mean_trade_standard_error",
        "sample_stddev(trade.net_pnl) / sqrt(trade_count)",
        "account_currency/trade",
        "trade",
        "none",
        "undefined with fewer than 2 trades"
    ),
    metric!(
        "time_in_market",
        "union_duration(open_trade_intervals) / observed_equity_duration",
        "ratio",
        "run",
        "none",
        "undefined when observed duration is zero"
    ),
    metric!(
        "average_holding_period",
        "mean(exit_time - entry_time)",
        "nanoseconds/trade",
        "trade",
        "none",
        "undefined when no closed trades"
    ),
    metric!(
        "closed_trade_count",
        "count(closed_trade)",
        "trades",
        "run",
        "none",
        "0 when no closed trades"
    ),
    // ── Return & profit ────────────────────────────────────────────
    metric!(
        "cagr",
        "(final_equity / initial_equity)^(365.2425 days / observed_duration) - 1",
        "ratio",
        "annual",
        "compounded to 365.2425-day years",
        "undefined when the observed duration or either equity endpoint is non-positive"
    ),
    metric!(
        "return_on_max_drawdown",
        "net_profit / max_drawdown_absolute",
        "ratio",
        "run",
        "none",
        "undefined when the curve never drew down"
    ),
    metric!(
        "average_trade_percent",
        "mean(closed_trade.net_pnl / closed_trade.entry_notional)",
        "ratio/trade",
        "trade",
        "none",
        "undefined when no closed trade has a positive entry notional"
    ),
    // ── Risk & drawdown ────────────────────────────────────────────
    metric!(
        "average_drawdown",
        "mean(running_peak_equity - equity over points strictly below their peak)",
        "account_currency",
        "mark_to_market",
        "none",
        "undefined when the curve never drew down"
    ),
    metric!(
        "max_time_to_recovery",
        "max(time from an equity peak until that peak is regained), completed recoveries only",
        "nanoseconds",
        "mark_to_market",
        "none",
        "0 when no drawdown was ever recovered"
    ),
    metric!(
        "max_trade_adverse_excursion",
        "max(closed_trade.mae)",
        "account_currency",
        "trade",
        "none",
        "undefined when no closed trades"
    ),
    // ── Ratios ─────────────────────────────────────────────────────
    metric!(
        "sortino_ratio",
        "mean(daily_return) / sqrt(mean(min(daily_return, 0)^2)) * sqrt(252), risk_free=0",
        "ratio",
        "daily",
        "sqrt(252)",
        "undefined with fewer than 2 returns or no downside deviation"
    ),
    metric!(
        "calmar_ratio",
        "cagr / max_drawdown_percent",
        "ratio",
        "annual",
        "numerator only",
        "undefined when CAGR or max drawdown percent is undefined, or drawdown is zero"
    ),
    metric!(
        "sterling_ratio",
        "cagr / (max_drawdown_percent + 0.10)",
        "ratio",
        "annual",
        "numerator only",
        "undefined when CAGR or max drawdown percent is undefined"
    ),
    metric!(
        "equity_curve_r_squared",
        "coefficient of determination of equity against elapsed time, ordinary least squares",
        "ratio",
        "mark_to_market",
        "none",
        "undefined with fewer than 3 points or zero variance in time or equity"
    ),
    metric!(
        "k_ratio",
        "ols_slope(equity vs elapsed days) / (stderr(slope) * observation_count)",
        "ratio",
        "mark_to_market",
        "none",
        "undefined with fewer than 3 points or a degenerate fit"
    ),
    // ── Trade-level excursions ─────────────────────────────────────
    metric!(
        "average_mae",
        "mean(closed_trade.mae)",
        "account_currency/trade",
        "trade",
        "none",
        "undefined when no closed trades"
    ),
    metric!(
        "average_mfe",
        "mean(closed_trade.mfe)",
        "account_currency/trade",
        "trade",
        "none",
        "undefined when no closed trades"
    ),
    metric!(
        "average_capture_efficiency",
        "mean(closed_trade.net_pnl / closed_trade.mfe) over trades with a positive MFE",
        "ratio/trade",
        "trade",
        "none",
        "undefined when no closed trade had a favourable excursion"
    ),
    // ── Exposure & activity ────────────────────────────────────────
    metric!(
        "long_trade_count",
        "count(closed_trade where direction = long)",
        "trades",
        "run",
        "none",
        "0 when no long trades"
    ),
    metric!(
        "short_trade_count",
        "count(closed_trade where direction = short)",
        "trades",
        "run",
        "none",
        "0 when no short trades"
    ),
    metric!(
        "max_concurrent_positions",
        "max(count of symbols holding non-zero exposure simultaneously)",
        "positions",
        "run",
        "none",
        "0 when nothing was ever held"
    ),
    metric!(
        "trades_per_year",
        "closed_trade_count / (observed_duration / 365.2425 days)",
        "trades/year",
        "annual",
        "365.2425-day years",
        "undefined when the observed duration is zero"
    ),
    metric!(
        "turnover",
        "sum(fill.quantity * fill.fill_price) / initial_equity",
        "ratio",
        "run",
        "none",
        "undefined when initial equity is zero"
    ),
    // ── Distribution & tails ───────────────────────────────────────
    metric!(
        "trade_pnl_skewness",
        "sample skewness (adjusted Fisher-Pearson G1) of closed_trade.net_pnl",
        "ratio",
        "trade",
        "none",
        "undefined with fewer than 3 trades or zero variance"
    ),
    metric!(
        "trade_pnl_excess_kurtosis",
        "sample excess kurtosis (G2) of closed_trade.net_pnl",
        "ratio",
        "trade",
        "none",
        "undefined with fewer than 4 trades or zero variance"
    ),
    metric!(
        "max_consecutive_wins",
        "longest run of closed trades with net_pnl > 0",
        "trades",
        "trade",
        "none",
        "0 when no winning trades"
    ),
    metric!(
        "max_consecutive_losses",
        "longest run of closed trades with net_pnl < 0",
        "trades",
        "trade",
        "none",
        "0 when no losing trades"
    ),
    metric!(
        "daily_value_at_risk_95",
        "negated 5th percentile of daily returns, lower-nearest-rank",
        "ratio",
        "daily",
        "none",
        "undefined with fewer than 2 daily returns"
    ),
    metric!(
        "daily_conditional_value_at_risk_95",
        "negated mean of daily returns at or below the 5th percentile",
        "ratio",
        "daily",
        "none",
        "undefined with fewer than 2 daily returns"
    ),
    metric!(
        "worst_daily_return",
        "min(daily_return)",
        "ratio",
        "daily",
        "none",
        "undefined when no daily return exists"
    ),
    metric!(
        "tail_ratio",
        "95th percentile of daily returns / abs(5th percentile of daily returns)",
        "ratio",
        "daily",
        "none",
        "undefined with fewer than 2 daily returns or a zero lower tail"
    ),
    // ── Stability ──────────────────────────────────────────────────
    metric!(
        "top_decile_pnl_share",
        "sum(top ceil(n/10) trades by net_pnl) / gross_profit",
        "ratio",
        "trade",
        "none",
        "undefined when there is no gross profit to concentrate"
    ),
];

pub fn metric_registry() -> &'static [MetricDefinition] {
    REGISTRY
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeDirection {
    Long,
    Short,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosedTrade {
    pub trade_id: u64,
    pub symbol: SymbolId,
    pub direction: TradeDirection,
    pub entry_time_ns: i64,
    pub exit_time_ns: i64,
    pub quantity: f64,
    pub average_entry_price: f64,
    pub average_exit_price: f64,
    pub net_pnl: f64,
    pub commission: f64,
    pub mae: f64,
    pub mfe: f64,
    pub capture_efficiency: Option<f64>,
    pub r_multiple: MetricValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnderwaterPoint {
    pub time_ns: i64,
    pub equity: f64,
    pub peak_equity: f64,
    pub drawdown: f64,
    pub drawdown_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarPoint {
    pub bucket: i64,
    pub closing_time_ns: i64,
    pub closing_equity: f64,
    pub change: f64,
    pub return_fraction: Option<f64>,
}

/// Mark-to-market equity resampled onto real calendar periods (§9.2). Every
/// series closes on the last observed mark inside each bucket, so the changes
/// telescope back to `final_equity - initial_equity` at any granularity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarEquity {
    pub daily: Vec<CalendarPoint>,
    pub weekly: Vec<CalendarPoint>,
    pub monthly: Vec<CalendarPoint>,
    pub annual: Vec<CalendarPoint>,
}

/// Execution-quality counters that explain *why* the headline numbers look the
/// way they do (§9.2 "Diagnostics"). These are counts and sums taken straight
/// from the ledger, never estimates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostics {
    pub commission_cost: f64,
    pub spread_cost: f64,
    pub slippage_cost: f64,
    /// Gross PnL before any of the three cost components above.
    pub gross_pnl_before_costs: f64,
    pub cost_share_of_gross_pnl: MetricValue,
    pub fill_count: u64,
    pub rejected_order_count: u64,
    pub cancelled_order_count: u64,
    pub unfilled_pending_order_count: u64,
    pub open_position_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricResult {
    pub id: String,
    pub value: MetricValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UncertaintyReport {
    /// Monte-Carlo confidence intervals are an M4 dependency. This typed value
    /// prevents M2 reports from presenting unlabeled point estimates.
    pub headline_confidence_intervals: DeferredUncertainty,
    pub mean_trade_standard_error: MetricValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeferredUncertainty {
    UnavailableUntilM4,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategyAnalysis {
    pub metrics_version: String,
    pub metrics: Vec<MetricResult>,
    pub trades: Vec<ClosedTrade>,
    pub underwater_curve: Vec<UnderwaterPoint>,
    pub calendar: CalendarEquity,
    pub diagnostics: Diagnostics,
    pub uncertainty: UncertaintyReport,
}

impl StrategyAnalysis {
    pub fn metric(&self, id: &str) -> Option<&MetricValue> {
        self.metrics
            .iter()
            .find(|metric| metric.id == id)
            .map(|metric| &metric.value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetricsError {
    InvalidInitialEquity,
    NonFinite {
        field: &'static str,
    },
    Unordered {
        field: &'static str,
    },
    UnknownSymbol {
        symbol: usize,
    },
    TooMany {
        field: &'static str,
        limit: usize,
        found: usize,
    },
    InvalidValue {
        field: &'static str,
    },
    ArithmeticOverflow {
        field: &'static str,
    },
    Inconsistent {
        field: &'static str,
    },
}
impl fmt::Display for MetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid metric input: {self:?}")
    }
}
impl Error for MetricsError {}

#[derive(Default)]
struct OpenTrade {
    direction: Option<TradeDirection>,
    entry_time_ns: i64,
    quantity: f64,
    entry_notional: f64,
    exit_quantity: f64,
    exit_notional: f64,
    pnl: f64,
    commission: f64,
}

pub fn analyze_simulation(
    report: &SimulationReport,
    streams: &[SymbolStream],
    initial_equity: f64,
) -> Result<StrategyAnalysis, MetricsError> {
    if !initial_equity.is_finite() || initial_equity < 0.0 {
        return Err(MetricsError::InvalidInitialEquity);
    }
    validate_report(report)?;
    let trades = attribute_trades(report, streams)?;
    let underwater_curve = underwater(&report.equity_curve);
    let calendar = resample_calendar(&report.equity_curve, initial_equity);
    let diagnostics = diagnostics(report);
    let values = compute_metrics(
        &trades,
        &underwater_curve,
        &calendar,
        report,
        initial_equity,
    );
    let sem = values
        .get("mean_trade_standard_error")
        .cloned()
        .unwrap_or_else(|| MetricValue::undefined(UndefinedReason::InsufficientObservations));
    let metrics = REGISTRY
        .iter()
        .map(|definition| MetricResult {
            id: definition.id.to_string(),
            value: values.get(definition.id).cloned().unwrap_or_else(|| {
                MetricValue::undefined(UndefinedReason::InsufficientObservations)
            }),
        })
        .collect();
    Ok(StrategyAnalysis {
        metrics_version: METRICS_SCHEMA_VERSION.to_string(),
        metrics,
        trades,
        underwater_curve,
        calendar,
        diagnostics,
        uncertainty: UncertaintyReport {
            headline_confidence_intervals: DeferredUncertainty::UnavailableUntilM4,
            mean_trade_standard_error: sem,
        },
    })
}

fn validate_report(report: &SimulationReport) -> Result<(), MetricsError> {
    for (field, found) in [
        ("fills", report.fills.len()),
        ("equity_curve", report.equity_curve.len()),
        ("events", report.events.len()),
    ] {
        if found > MAX_METRIC_INPUT_ITEMS {
            return Err(MetricsError::TooMany {
                field,
                limit: MAX_METRIC_INPUT_ITEMS,
                found,
            });
        }
    }
    let scalars = [
        ("final_cash", report.final_cash),
        ("final_equity", report.final_equity),
        ("final_realized_pnl", report.final_realized_pnl),
        ("total_commission", report.total_commission),
    ];
    if let Some((field, _)) = scalars.into_iter().find(|(_, value)| !value.is_finite()) {
        return Err(MetricsError::NonFinite { field });
    }
    if report
        .equity_curve
        .windows(2)
        .any(|pair| (pair[0].time_ns, pair[0].sequence) > (pair[1].time_ns, pair[1].sequence))
    {
        return Err(MetricsError::Unordered {
            field: "equity_curve",
        });
    }
    if report
        .fills
        .windows(2)
        .any(|pair| (pair[0].time_ns, pair[0].sequence) > (pair[1].time_ns, pair[1].sequence))
    {
        return Err(MetricsError::Unordered { field: "fills" });
    }
    for point in &report.equity_curve {
        if !point.equity.is_finite() || !point.cash.is_finite() {
            return Err(MetricsError::NonFinite {
                field: "equity_curve",
            });
        }
    }
    // The simulator closes every run with a final mark-to-market point, so a
    // headline `final_equity` that disagrees with the curve's last value would
    // silently split `total_return` from the drawdown and calendar series.
    if let Some(last) = report.equity_curve.last()
        && last.equity != report.final_equity
    {
        return Err(MetricsError::Inconsistent {
            field: "final_equity",
        });
    }
    for fill in &report.fills {
        let values = [
            fill.quantity,
            fill.fill_price,
            fill.realized_pnl,
            fill.commission,
            fill.position_units_after,
        ];
        if values.into_iter().any(|value| !value.is_finite()) {
            return Err(MetricsError::NonFinite { field: "fills" });
        }
        if fill.symbol.0 >= report.symbols.len() {
            return Err(MetricsError::UnknownSymbol {
                symbol: fill.symbol.0,
            });
        }
        if fill.quantity <= 0.0 || fill.commission < 0.0 {
            return Err(MetricsError::InvalidValue { field: "fills" });
        }
    }
    Ok(())
}

fn attribute_trades(
    report: &SimulationReport,
    streams: &[SymbolStream],
) -> Result<Vec<ClosedTrade>, MetricsError> {
    let bar_count = streams.iter().try_fold(0_usize, |total, stream| {
        total
            .checked_add(stream.bars.len())
            .ok_or(MetricsError::TooMany {
                field: "streams.bars",
                limit: MAX_METRIC_INPUT_ITEMS,
                found: usize::MAX,
            })
    })?;
    if bar_count > MAX_METRIC_INPUT_ITEMS {
        return Err(MetricsError::TooMany {
            field: "streams.bars",
            limit: MAX_METRIC_INPUT_ITEMS,
            found: bar_count,
        });
    }
    for stream in streams {
        for bar in &stream.bars {
            if bar.open_time_ns > bar.close_time_ns
                || [bar.open, bar.high, bar.low, bar.close, bar.volume]
                    .into_iter()
                    .any(|value| !value.is_finite())
            {
                return Err(MetricsError::InvalidValue {
                    field: "streams.bars",
                });
            }
        }
    }
    let mut open: BTreeMap<usize, OpenTrade> = BTreeMap::new();
    let mut trades = Vec::new();
    for fill in &report.fills {
        let signed = match fill.side {
            OrderSide::Buy => fill.quantity,
            OrderSide::Sell => -fill.quantity,
        };
        let before = fill.position_units_after - signed;
        let after = fill.position_units_after;
        let before_sign = before.total_cmp(&0.0);
        let after_sign = after.total_cmp(&0.0);
        if before == 0.0 && after != 0.0 {
            open.insert(
                fill.symbol.0,
                OpenTrade {
                    direction: Some(if after > 0.0 {
                        TradeDirection::Long
                    } else {
                        TradeDirection::Short
                    }),
                    entry_time_ns: fill.time_ns,
                    quantity: after.abs(),
                    entry_notional: fill.fill_price * after.abs(),
                    commission: fill.commission,
                    ..OpenTrade::default()
                },
            );
            continue;
        }
        if before == 0.0 {
            // Neither opens nor reduces exposure; attributing it would leave a
            // phantom open trade behind that never closes.
            continue;
        }
        let state = open.entry(fill.symbol.0).or_default();
        if after != 0.0 && before_sign == after_sign && after.abs() > before.abs() {
            state.quantity += fill.quantity;
            state.entry_notional += fill.fill_price * fill.quantity;
            state.commission += fill.commission;
            continue;
        }
        {
            let closing_quantity = fill.quantity.min(before.abs());
            let fee_share = if fill.quantity == 0.0 {
                0.0
            } else {
                fill.commission * closing_quantity / fill.quantity
            };
            state.exit_quantity += closing_quantity;
            state.exit_notional += fill.fill_price * closing_quantity;
            state.pnl += fill.realized_pnl;
            state.commission += fee_share;
            if after == 0.0 || before_sign != after_sign {
                let completed = open.remove(&fill.symbol.0).unwrap_or_default();
                trades.push(close_trade(
                    trades.len() as u64,
                    fill.symbol,
                    fill.time_ns,
                    completed,
                    streams,
                )?);
                if after != 0.0 {
                    let opening_quantity = after.abs();
                    open.insert(
                        fill.symbol.0,
                        OpenTrade {
                            direction: Some(if after > 0.0 {
                                TradeDirection::Long
                            } else {
                                TradeDirection::Short
                            }),
                            entry_time_ns: fill.time_ns,
                            quantity: opening_quantity,
                            entry_notional: fill.fill_price * opening_quantity,
                            commission: fill.commission - fee_share,
                            ..OpenTrade::default()
                        },
                    );
                }
            }
        }
    }
    Ok(trades)
}

fn close_trade(
    id: u64,
    symbol: SymbolId,
    exit_time_ns: i64,
    state: OpenTrade,
    streams: &[SymbolStream],
) -> Result<ClosedTrade, MetricsError> {
    let direction = state.direction.unwrap_or(TradeDirection::Long);
    let entry = state.entry_notional / state.quantity;
    let exit = if state.exit_quantity == 0.0 {
        entry
    } else {
        state.exit_notional / state.exit_quantity
    };
    // `SymbolId` is an index into the report's symbol table, which the simulator
    // builds from these streams in order. Duplicate display names must not make
    // excursions resolve against the wrong series, so index — never name — wins.
    let stream = streams.get(symbol.0);
    let mut adverse_per_unit: f64 = 0.0;
    let mut favorable_per_unit: f64 = 0.0;
    if let Some(stream) = stream {
        for bar in &stream.bars {
            if bar.open_time_ns >= state.entry_time_ns && bar.close_time_ns <= exit_time_ns {
                match direction {
                    TradeDirection::Long => {
                        adverse_per_unit = adverse_per_unit.max(entry - bar.low);
                        favorable_per_unit = favorable_per_unit.max(bar.high - entry);
                    }
                    TradeDirection::Short => {
                        adverse_per_unit = adverse_per_unit.max(bar.high - entry);
                        favorable_per_unit = favorable_per_unit.max(entry - bar.low);
                    }
                }
            }
        }
    }
    let net_pnl = state.pnl - state.commission;
    let mfe = favorable_per_unit * state.quantity;
    let mae = adverse_per_unit * state.quantity;
    let capture_efficiency = (mfe > 0.0).then_some(net_pnl / mfe);
    if [entry, exit, net_pnl, state.commission, mae, mfe]
        .into_iter()
        .chain(capture_efficiency)
        .any(|value| !value.is_finite())
    {
        return Err(MetricsError::ArithmeticOverflow {
            field: "closed_trade",
        });
    }
    Ok(ClosedTrade {
        trade_id: id,
        symbol,
        direction,
        entry_time_ns: state.entry_time_ns,
        exit_time_ns,
        quantity: state.quantity,
        average_entry_price: entry,
        average_exit_price: exit,
        net_pnl,
        commission: state.commission,
        mae,
        mfe,
        capture_efficiency,
        r_multiple: MetricValue::undefined(UndefinedReason::MissingInitialRisk),
    })
}

fn underwater(points: &[crate::core::strategy_simulator::EquityPoint]) -> Vec<UnderwaterPoint> {
    let mut peak = f64::NEG_INFINITY;
    points
        .iter()
        .map(|point| {
            peak = peak.max(point.equity);
            let drawdown = (peak - point.equity).max(0.0);
            UnderwaterPoint {
                time_ns: point.time_ns,
                equity: point.equity,
                peak_equity: peak,
                drawdown,
                drawdown_percent: (peak > 0.0).then_some(drawdown / peak),
            }
        })
        .collect()
}

fn resample_calendar(
    points: &[crate::core::strategy_simulator::EquityPoint],
    initial: f64,
) -> CalendarEquity {
    CalendarEquity {
        daily: resample(points, initial, |time| time.div_euclid(DAY_NS)),
        // Buckets by ISO week, so a week's close is its last mark regardless of
        // which weekday the run happens to end on.
        weekly: resample(points, initial, |time| {
            utc_date(time).map_or(time.div_euclid(7 * DAY_NS), |date| {
                let week = date.iso_week();
                i64::from(week.year()) * 100 + i64::from(week.week())
            })
        }),
        monthly: resample(points, initial, |time| {
            utc_date(time).map_or(time.div_euclid(30 * DAY_NS), |date| {
                i64::from(date.year()) * 100 + i64::from(date.month())
            })
        }),
        annual: resample(points, initial, |time| {
            utc_date(time).map_or(time.div_euclid(365 * DAY_NS), |date| i64::from(date.year()))
        }),
    }
}

fn utc_date(time_ns: i64) -> Option<chrono::NaiveDate> {
    chrono::DateTime::from_timestamp_nanos(time_ns)
        .naive_utc()
        .date()
        .into()
}

/// Collapses the mark-to-market curve onto one closing observation per bucket.
/// Points arrive time-ordered, so the last write per bucket is that period's
/// close and the resulting changes telescope to the overall equity change.
fn resample(
    points: &[crate::core::strategy_simulator::EquityPoint],
    initial: f64,
    bucket_of: impl Fn(i64) -> i64,
) -> Vec<CalendarPoint> {
    let mut closes: BTreeMap<i64, (i64, f64)> = BTreeMap::new();
    for point in points {
        closes.insert(bucket_of(point.time_ns), (point.time_ns, point.equity));
    }
    let mut previous = initial;
    closes
        .into_iter()
        .map(|(bucket, (time, equity))| {
            let change = equity - previous;
            let return_fraction = (previous != 0.0).then_some(change / previous);
            previous = equity;
            CalendarPoint {
                bucket,
                closing_time_ns: time,
                closing_equity: equity,
                change,
                return_fraction,
            }
        })
        .collect()
}

fn diagnostics(report: &SimulationReport) -> Diagnostics {
    let spread_cost: f64 = report.fills.iter().map(|fill| fill.spread_cost).sum();
    let slippage_cost: f64 = report.fills.iter().map(|fill| fill.slippage_cost).sum();
    let commission_cost: f64 = report.fills.iter().map(|fill| fill.commission).sum();
    // `final_realized_pnl` is already net of the three cost components, so gross
    // is recovered by adding them back rather than re-deriving from fills.
    let costs = spread_cost + slippage_cost + commission_cost;
    let gross_pnl_before_costs = report.final_realized_pnl + costs;
    Diagnostics {
        commission_cost,
        spread_cost,
        slippage_cost,
        gross_pnl_before_costs,
        cost_share_of_gross_pnl: if gross_pnl_before_costs == 0.0 {
            MetricValue::undefined(UndefinedReason::ZeroDenominator)
        } else {
            MetricValue::defined(costs / gross_pnl_before_costs.abs())
        },
        fill_count: report.fills.len() as u64,
        rejected_order_count: report.rejections.len() as u64,
        cancelled_order_count: report.cancellations.len() as u64,
        unfilled_pending_order_count: report.pending_orders.len() as u64,
        open_position_count: report
            .positions
            .iter()
            .filter(|position| position.units != 0.0)
            .count() as u64,
    }
}

fn compute_metrics(
    trades: &[ClosedTrade],
    underwater: &[UnderwaterPoint],
    calendar: &CalendarEquity,
    report: &SimulationReport,
    initial: f64,
) -> BTreeMap<&'static str, MetricValue> {
    let mut out = BTreeMap::new();
    let pnls: Vec<f64> = trades.iter().map(|trade| trade.net_pnl).collect();
    let gross_profit: f64 = pnls.iter().copied().filter(|value| *value > 0.0).sum();
    let gross_loss: f64 = pnls.iter().copied().filter(|value| *value < 0.0).sum();
    let net: f64 = pnls.iter().sum();
    out.insert("net_profit", MetricValue::defined(net));
    out.insert("gross_profit", MetricValue::defined(gross_profit));
    out.insert("gross_loss", MetricValue::defined(gross_loss));
    out.insert(
        "total_return",
        if initial == 0.0 {
            MetricValue::undefined(UndefinedReason::ZeroDenominator)
        } else {
            MetricValue::defined((report.final_equity - initial) / initial)
        },
    );
    out.insert("average_trade", mean(&pnls));
    out.insert(
        "profit_factor",
        if gross_loss == 0.0 {
            MetricValue::undefined(UndefinedReason::NoLosingTrades)
        } else {
            MetricValue::defined(gross_profit / -gross_loss)
        },
    );
    let wins: Vec<f64> = pnls.iter().copied().filter(|value| *value > 0.0).collect();
    let losses: Vec<f64> = pnls.iter().copied().filter(|value| *value < 0.0).collect();
    // Probability-weighted per-trade edge. This is *not* `average_trade` whenever
    // scratch trades exist: scratches count in the denominator of both rates but
    // contribute to neither conditional mean.
    out.insert(
        "expectancy",
        if pnls.is_empty() {
            MetricValue::undefined(UndefinedReason::NoTrades)
        } else {
            let total = pnls.len() as f64;
            let win_leg =
                numeric_mean(&wins).map_or(0.0, |average| average * wins.len() as f64 / total);
            let loss_leg =
                numeric_mean(&losses).map_or(0.0, |average| average * losses.len() as f64 / total);
            MetricValue::defined(win_leg + loss_leg)
        },
    );
    out.insert(
        "payoff_ratio",
        match (numeric_mean(&wins), numeric_mean(&losses)) {
            (Some(win), Some(loss)) => MetricValue::defined(win / -loss),
            (None, _) => MetricValue::undefined(UndefinedReason::NoWinningTrades),
            (_, None) => MetricValue::undefined(UndefinedReason::NoLosingTrades),
        },
    );
    let max_dd = underwater
        .iter()
        .map(|point| point.drawdown)
        .fold(0.0, f64::max);
    out.insert("max_drawdown_absolute", MetricValue::defined(max_dd));
    let percentages: Vec<f64> = underwater
        .iter()
        .filter_map(|point| point.drawdown_percent)
        .collect();
    out.insert(
        "max_drawdown_percent",
        percentages.iter().copied().reduce(f64::max).map_or_else(
            || MetricValue::undefined(UndefinedReason::ZeroDenominator),
            MetricValue::defined,
        ),
    );
    out.insert(
        "ulcer_index",
        numeric_mean(
            &percentages
                .iter()
                .map(|value| value * value)
                .collect::<Vec<_>>(),
        )
        .map(|value| MetricValue::defined(value.sqrt()))
        .unwrap_or_else(|| MetricValue::undefined(UndefinedReason::ZeroDenominator)),
    );
    let (dd_duration, stagnation) = durations(underwater);
    out.insert(
        "max_drawdown_duration",
        MetricValue::defined(dd_duration as f64),
    );
    out.insert(
        "longest_stagnation",
        MetricValue::defined(stagnation as f64),
    );
    let returns: Vec<f64> = calendar
        .daily
        .iter()
        .filter_map(|point| point.return_fraction)
        .collect();
    out.insert("sharpe_ratio", annualized_sharpe(&returns));
    out.insert("mean_trade_standard_error", standard_error(&pnls));
    let observed = report
        .equity_curve
        .first()
        .zip(report.equity_curve.last())
        .map_or(0, |(a, b)| b.time_ns.saturating_sub(a.time_ns));
    let held = union_duration(&exposure_intervals(report)).min(observed);
    out.insert(
        "time_in_market",
        if observed == 0 {
            MetricValue::undefined(UndefinedReason::ZeroDenominator)
        } else {
            MetricValue::defined(held as f64 / observed as f64)
        },
    );
    let total_holding: i128 = trades
        .iter()
        .map(|trade| i128::from(trade.exit_time_ns) - i128::from(trade.entry_time_ns))
        .sum();
    out.insert(
        "average_holding_period",
        if trades.is_empty() {
            MetricValue::undefined(UndefinedReason::NoTrades)
        } else {
            MetricValue::defined(total_holding as f64 / trades.len() as f64)
        },
    );
    out.insert(
        "closed_trade_count",
        MetricValue::defined(trades.len() as f64),
    );

    // ── Return & profit ────────────────────────────────────────────
    let years = observed as f64 / YEAR_NS as f64;
    let cagr = if observed <= 0 || initial <= 0.0 || report.final_equity <= 0.0 {
        MetricValue::undefined(UndefinedReason::ZeroDenominator)
    } else {
        MetricValue::defined((report.final_equity / initial).powf(1.0 / years) - 1.0)
    };
    out.insert("cagr", cagr.clone());
    out.insert(
        "return_on_max_drawdown",
        if max_dd == 0.0 {
            MetricValue::undefined(UndefinedReason::ZeroDenominator)
        } else {
            MetricValue::defined(net / max_dd)
        },
    );
    let trade_returns: Vec<f64> = trades
        .iter()
        .filter(|trade| trade.quantity * trade.average_entry_price > 0.0)
        .map(|trade| trade.net_pnl / (trade.quantity * trade.average_entry_price))
        .collect();
    out.insert(
        "average_trade_percent",
        defined_or(&trade_returns, UndefinedReason::NoTrades),
    );

    // ── Risk & drawdown ────────────────────────────────────────────
    let active_drawdowns: Vec<f64> = underwater
        .iter()
        .map(|point| point.drawdown)
        .filter(|drawdown| *drawdown > 0.0)
        .collect();
    out.insert(
        "average_drawdown",
        defined_or(&active_drawdowns, UndefinedReason::ZeroDenominator),
    );
    out.insert(
        "max_time_to_recovery",
        MetricValue::defined(max_time_to_recovery(underwater) as f64),
    );
    let maes: Vec<f64> = trades.iter().map(|trade| trade.mae).collect();
    let mfes: Vec<f64> = trades.iter().map(|trade| trade.mfe).collect();
    out.insert(
        "max_trade_adverse_excursion",
        maes.iter().copied().reduce(f64::max).map_or_else(
            || MetricValue::undefined(UndefinedReason::NoTrades),
            MetricValue::defined,
        ),
    );

    // ── Ratios ─────────────────────────────────────────────────────
    out.insert("sortino_ratio", annualized_sortino(&returns));
    let max_dd_percent = match out.get("max_drawdown_percent") {
        Some(MetricValue::Defined { value }) => Some(*value),
        _ => None,
    };
    let annual_return = match &cagr {
        MetricValue::Defined { value } => Some(*value),
        MetricValue::Undefined { .. } => None,
    };
    out.insert(
        "calmar_ratio",
        match (annual_return, max_dd_percent) {
            (Some(_), Some(drawdown)) if drawdown == 0.0 => {
                MetricValue::undefined(UndefinedReason::ZeroDenominator)
            }
            (Some(annual), Some(drawdown)) => MetricValue::defined(annual / drawdown),
            _ => MetricValue::undefined(UndefinedReason::ZeroDenominator),
        },
    );
    out.insert(
        "sterling_ratio",
        match (annual_return, max_dd_percent) {
            (Some(annual), Some(drawdown)) => MetricValue::defined(annual / (drawdown + 0.10)),
            _ => MetricValue::undefined(UndefinedReason::ZeroDenominator),
        },
    );
    let (r_squared, k_ratio) = equity_curve_fit(&report.equity_curve);
    out.insert("equity_curve_r_squared", r_squared);
    out.insert("k_ratio", k_ratio);

    // ── Trade-level excursions ─────────────────────────────────────
    out.insert("average_mae", defined_or(&maes, UndefinedReason::NoTrades));
    out.insert("average_mfe", defined_or(&mfes, UndefinedReason::NoTrades));
    let captures: Vec<f64> = trades
        .iter()
        .filter_map(|trade| trade.capture_efficiency)
        .collect();
    out.insert(
        "average_capture_efficiency",
        defined_or(&captures, UndefinedReason::NoTrades),
    );

    // ── Exposure & activity ────────────────────────────────────────
    let longs = trades
        .iter()
        .filter(|trade| trade.direction == TradeDirection::Long)
        .count();
    out.insert("long_trade_count", MetricValue::defined(longs as f64));
    out.insert(
        "short_trade_count",
        MetricValue::defined((trades.len() - longs) as f64),
    );
    out.insert(
        "max_concurrent_positions",
        MetricValue::defined(max_concurrent_positions(report) as f64),
    );
    out.insert(
        "trades_per_year",
        if observed == 0 {
            MetricValue::undefined(UndefinedReason::ZeroDenominator)
        } else {
            MetricValue::defined(trades.len() as f64 / years)
        },
    );
    let traded_notional: f64 = report
        .fills
        .iter()
        .map(|fill| fill.quantity * fill.fill_price)
        .sum();
    out.insert(
        "turnover",
        if initial == 0.0 {
            MetricValue::undefined(UndefinedReason::ZeroDenominator)
        } else {
            MetricValue::defined(traded_notional / initial)
        },
    );

    // ── Distribution & tails ───────────────────────────────────────
    out.insert("trade_pnl_skewness", skewness(&pnls));
    out.insert("trade_pnl_excess_kurtosis", excess_kurtosis(&pnls));
    let (win_streak, loss_streak) = streaks(&pnls);
    out.insert(
        "max_consecutive_wins",
        MetricValue::defined(win_streak as f64),
    );
    out.insert(
        "max_consecutive_losses",
        MetricValue::defined(loss_streak as f64),
    );
    let lower_tail = percentile(&returns, 0.05);
    let upper_tail = percentile(&returns, 0.95);
    out.insert(
        "daily_value_at_risk_95",
        lower_tail.map_or_else(
            || MetricValue::undefined(UndefinedReason::InsufficientObservations),
            |value| MetricValue::defined(-value),
        ),
    );
    out.insert(
        "daily_conditional_value_at_risk_95",
        match lower_tail {
            None => MetricValue::undefined(UndefinedReason::InsufficientObservations),
            Some(threshold) => {
                let tail: Vec<f64> = returns
                    .iter()
                    .copied()
                    .filter(|value| *value <= threshold)
                    .collect();
                numeric_mean(&tail).map_or_else(
                    || MetricValue::undefined(UndefinedReason::InsufficientObservations),
                    |value| MetricValue::defined(-value),
                )
            }
        },
    );
    out.insert(
        "worst_daily_return",
        returns.iter().copied().reduce(f64::min).map_or_else(
            || MetricValue::undefined(UndefinedReason::InsufficientObservations),
            MetricValue::defined,
        ),
    );
    out.insert(
        "tail_ratio",
        match (upper_tail, lower_tail) {
            (Some(upper), Some(lower)) if lower != 0.0 => MetricValue::defined(upper / lower.abs()),
            (Some(_), Some(_)) => MetricValue::undefined(UndefinedReason::ZeroDenominator),
            _ => MetricValue::undefined(UndefinedReason::InsufficientObservations),
        },
    );

    // ── Stability ──────────────────────────────────────────────────
    out.insert(
        "top_decile_pnl_share",
        if gross_profit <= 0.0 {
            MetricValue::undefined(UndefinedReason::NoWinningTrades)
        } else {
            let mut sorted = pnls.clone();
            sorted.sort_by(|a, b| b.total_cmp(a));
            let decile = pnls.len().div_ceil(10);
            MetricValue::defined(sorted.iter().take(decile).sum::<f64>() / gross_profit)
        },
    );
    out
}

fn defined_or(values: &[f64], reason: UndefinedReason) -> MetricValue {
    numeric_mean(values).map_or_else(|| MetricValue::undefined(reason), MetricValue::defined)
}

/// Longest peak-to-recovery span, counting only drawdowns that actually
/// recovered. An unrecovered tail is reported by `longest_stagnation`, not here,
/// so a still-underwater run cannot masquerade as a fast recoverer.
fn max_time_to_recovery(points: &[UnderwaterPoint]) -> i64 {
    let Some(first) = points.first() else {
        return 0;
    };
    let mut peak = first.equity;
    let mut peak_time = first.time_ns;
    let mut underwater_since: Option<i64> = None;
    let mut longest = 0_i64;
    for point in &points[1..] {
        if point.equity >= peak {
            if let Some(start) = underwater_since.take() {
                longest = longest.max(point.time_ns.saturating_sub(start));
            }
            peak = point.equity;
            peak_time = point.time_ns;
        } else if underwater_since.is_none() {
            underwater_since = Some(peak_time);
        }
    }
    longest
}

fn max_concurrent_positions(report: &SimulationReport) -> usize {
    let mut held: BTreeMap<usize, ()> = BTreeMap::new();
    let mut peak = 0;
    for fill in &report.fills {
        if fill.position_units_after == 0.0 {
            held.remove(&fill.symbol.0);
        } else {
            held.insert(fill.symbol.0, ());
        }
        peak = peak.max(held.len());
    }
    peak
}

/// Ordinary least squares of equity against elapsed days, returning the fit's
/// coefficient of determination and Kestner's K-ratio (slope over the slope's
/// standard error, normalized by the observation count).
fn equity_curve_fit(
    points: &[crate::core::strategy_simulator::EquityPoint],
) -> (MetricValue, MetricValue) {
    let insufficient = || MetricValue::undefined(UndefinedReason::InsufficientObservations);
    if points.len() < 3 {
        return (insufficient(), insufficient());
    }
    let origin = points[0].time_ns;
    let xs: Vec<f64> = points
        .iter()
        .map(|point| point.time_ns.saturating_sub(origin) as f64 / DAY_NS as f64)
        .collect();
    let ys: Vec<f64> = points.iter().map(|point| point.equity).collect();
    let count = xs.len() as f64;
    let mean_x = xs.iter().sum::<f64>() / count;
    let mean_y = ys.iter().sum::<f64>() / count;
    let sxx: f64 = xs.iter().map(|x| (x - mean_x).powi(2)).sum();
    let syy: f64 = ys.iter().map(|y| (y - mean_y).powi(2)).sum();
    let sxy: f64 = xs
        .iter()
        .zip(&ys)
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum();
    if sxx == 0.0 || syy == 0.0 {
        return (
            MetricValue::undefined(UndefinedReason::ZeroVariance),
            MetricValue::undefined(UndefinedReason::ZeroVariance),
        );
    }
    let slope = sxy / sxx;
    let residual: f64 = xs
        .iter()
        .zip(&ys)
        .map(|(x, y)| (y - (mean_y + slope * (x - mean_x))).powi(2))
        .sum();
    let r_squared = MetricValue::defined(1.0 - residual / syy);
    let slope_standard_error = (residual / (count - 2.0) / sxx).sqrt();
    let k_ratio = if slope_standard_error == 0.0 {
        MetricValue::undefined(UndefinedReason::ZeroVariance)
    } else {
        MetricValue::defined(slope / (slope_standard_error * count))
    };
    (r_squared, k_ratio)
}

fn annualized_sortino(values: &[f64]) -> MetricValue {
    if values.len() < 2 {
        return MetricValue::undefined(UndefinedReason::InsufficientObservations);
    }
    let mean = numeric_mean(values).unwrap_or(0.0);
    let downside =
        (values.iter().map(|v| v.min(0.0).powi(2)).sum::<f64>() / values.len() as f64).sqrt();
    if downside == 0.0 {
        MetricValue::undefined(UndefinedReason::ZeroVariance)
    } else {
        MetricValue::defined(mean / downside * YEAR_DAYS.sqrt())
    }
}

fn skewness(values: &[f64]) -> MetricValue {
    let count = values.len() as f64;
    if values.len() < 3 {
        return MetricValue::undefined(UndefinedReason::InsufficientObservations);
    }
    let mean = numeric_mean(values).unwrap_or(0.0);
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (count - 1.0);
    if variance == 0.0 {
        return MetricValue::undefined(UndefinedReason::ZeroVariance);
    }
    let m3 = values.iter().map(|v| (v - mean).powi(3)).sum::<f64>() / count;
    let biased = m3 / (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count).powf(1.5);
    MetricValue::defined((count * (count - 1.0)).sqrt() / (count - 2.0) * biased)
}

fn excess_kurtosis(values: &[f64]) -> MetricValue {
    let count = values.len() as f64;
    if values.len() < 4 {
        return MetricValue::undefined(UndefinedReason::InsufficientObservations);
    }
    let mean = numeric_mean(values).unwrap_or(0.0);
    let m2 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count;
    if m2 == 0.0 {
        return MetricValue::undefined(UndefinedReason::ZeroVariance);
    }
    let m4 = values.iter().map(|v| (v - mean).powi(4)).sum::<f64>() / count;
    let biased = m4 / (m2 * m2) - 3.0;
    MetricValue::defined(
        (count - 1.0) / ((count - 2.0) * (count - 3.0)) * ((count + 1.0) * biased + 6.0),
    )
}

fn streaks(pnls: &[f64]) -> (usize, usize) {
    let (mut best_win, mut best_loss, mut win, mut loss) = (0, 0, 0, 0);
    for pnl in pnls {
        // A scratch trade breaks both streaks: it is neither a win nor a loss.
        win = if *pnl > 0.0 { win + 1 } else { 0 };
        loss = if *pnl < 0.0 { loss + 1 } else { 0 };
        best_win = best_win.max(win);
        best_loss = best_loss.max(loss);
    }
    (best_win, best_loss)
}

/// Lower-nearest-rank percentile: no interpolation, so the value returned is
/// always one that actually occurred in the sample.
fn percentile(values: &[f64], fraction: f64) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (fraction * (sorted.len() - 1) as f64).round() as usize;
    sorted.get(rank.min(sorted.len() - 1)).copied()
}

fn mean(values: &[f64]) -> MetricValue {
    numeric_mean(values)
        .map(MetricValue::defined)
        .unwrap_or_else(|| MetricValue::undefined(UndefinedReason::NoTrades))
}
fn numeric_mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}
fn standard_error(values: &[f64]) -> MetricValue {
    if values.len() < 2 {
        return MetricValue::undefined(UndefinedReason::InsufficientObservations);
    }
    let mean = numeric_mean(values).unwrap_or(0.0);
    let variance =
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    MetricValue::defined(variance.sqrt() / (values.len() as f64).sqrt())
}
fn annualized_sharpe(values: &[f64]) -> MetricValue {
    if values.len() < 2 {
        return MetricValue::undefined(UndefinedReason::InsufficientObservations);
    }
    let mean = numeric_mean(values).unwrap_or(0.0);
    let variance =
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    if variance == 0.0 {
        MetricValue::undefined(UndefinedReason::ZeroVariance)
    } else {
        MetricValue::defined(mean / variance.sqrt() * YEAR_DAYS.sqrt())
    }
}

fn exposure_intervals(report: &SimulationReport) -> Vec<(i64, i64)> {
    let end = report.equity_curve.last().map_or(0, |point| point.time_ns);
    let mut opened = BTreeMap::new();
    let mut intervals = Vec::new();
    for fill in &report.fills {
        let signed = match fill.side {
            OrderSide::Buy => fill.quantity,
            OrderSide::Sell => -fill.quantity,
        };
        let before = fill.position_units_after - signed;
        if before == 0.0 && fill.position_units_after != 0.0 {
            opened.insert(fill.symbol.0, fill.time_ns);
        } else if before != 0.0
            && (fill.position_units_after == 0.0
                || before.signum() != fill.position_units_after.signum())
        {
            if let Some(start) = opened.remove(&fill.symbol.0) {
                intervals.push((start, fill.time_ns));
            }
            if fill.position_units_after != 0.0 {
                opened.insert(fill.symbol.0, fill.time_ns);
            }
        }
    }
    intervals.extend(opened.into_values().map(|start| (start, end)));
    intervals
}

fn union_duration(intervals: &[(i64, i64)]) -> i64 {
    let mut intervals = intervals.to_vec();
    intervals.sort_unstable();
    let Some(&(mut start, mut end)) = intervals.first() else {
        return 0;
    };
    let mut total = 0_i64;
    for &(next_start, next_end) in &intervals[1..] {
        if next_start <= end {
            end = end.max(next_end);
        } else {
            total = total.saturating_add(end.saturating_sub(start));
            start = next_start;
            end = next_end;
        }
    }
    total.saturating_add(end.saturating_sub(start))
}

fn durations(points: &[UnderwaterPoint]) -> (i64, i64) {
    let Some(first) = points.first() else {
        return (0, 0);
    };
    let mut peak = first.equity;
    let mut peak_time = first.time_ns;
    let mut max_below = 0;
    let mut max_stagnant = 0;
    for point in &points[1..] {
        if point.equity > peak {
            peak = point.equity;
            peak_time = point.time_ns;
            continue;
        }
        let duration = point.time_ns.saturating_sub(peak_time);
        max_stagnant = max_stagnant.max(duration);
        if point.equity < peak {
            max_below = max_below.max(duration);
        }
    }
    (max_below, max_stagnant)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::DAY_NS;
    use crate::core::strategy_metrics::{
        DeferredUncertainty, METRICS_SCHEMA_VERSION, MetricValue, UndefinedReason,
        analyze_simulation, metric_registry,
    };
    use crate::core::strategy_simulator::{
        ClientOrderId, EquityPoint, FillRecord, OrderSide, SimBar, SimulationReport, SymbolId,
        SymbolStream,
    };
    use std::collections::BTreeSet;

    pub(crate) fn fill(
        order: u64,
        time_ns: i64,
        side: OrderSide,
        quantity: f64,
        price: f64,
        realized_pnl: f64,
        position_units_after: f64,
    ) -> FillRecord {
        FillRecord {
            order_id: ClientOrderId(order),
            time_ns,
            sequence: order,
            symbol: SymbolId(0),
            side,
            quantity,
            reference_price: price,
            quoted_price: price,
            fill_price: price,
            spread_cost: 0.0,
            slippage_cost: 0.0,
            commission: 0.0,
            realized_pnl,
            cash_after: 1_000.0,
            position_units_after,
            avg_entry_after: if position_units_after == 0.0 {
                0.0
            } else {
                price
            },
        }
    }

    pub(crate) fn report(fills: Vec<FillRecord>, equity: &[(i64, f64)]) -> SimulationReport {
        SimulationReport {
            symbols: vec!["TEST".into()],
            events: vec![],
            fills,
            rejections: vec![],
            cancellations: vec![],
            pending_orders: vec![],
            positions: vec![],
            equity_curve: equity
                .iter()
                .enumerate()
                .map(|(sequence, (time_ns, value))| EquityPoint {
                    time_ns: *time_ns,
                    sequence: sequence as u64,
                    cash: *value,
                    equity: *value,
                })
                .collect(),
            final_cash: equity.last().map_or(1_000.0, |point| point.1),
            final_equity: equity.last().map_or(1_000.0, |point| point.1),
            final_realized_pnl: 0.0,
            total_commission: 0.0,
        }
    }

    fn streams() -> Vec<SymbolStream> {
        vec![SymbolStream {
            symbol: "TEST".into(),
            bars: vec![
                SimBar {
                    open_time_ns: 0,
                    close_time_ns: 9,
                    open: 100.0,
                    high: 101.0,
                    low: 99.0,
                    close: 100.0,
                    volume: 10.0,
                },
                SimBar {
                    open_time_ns: 10,
                    close_time_ns: 19,
                    open: 100.0,
                    high: 110.0,
                    low: 95.0,
                    close: 105.0,
                    volume: 10.0,
                },
                SimBar {
                    open_time_ns: 20,
                    close_time_ns: 29,
                    open: 105.0,
                    high: 108.0,
                    low: 90.0,
                    close: 92.0,
                    volume: 10.0,
                },
                SimBar {
                    open_time_ns: 30,
                    close_time_ns: 39,
                    open: 92.0,
                    high: 96.0,
                    low: 88.0,
                    close: 95.0,
                    volume: 10.0,
                },
            ],
        }]
    }

    #[test]
    fn registry_defines_every_metric_contract_without_duplicate_ids() {
        assert_eq!(METRICS_SCHEMA_VERSION, "strategy-metrics/v1");
        let registry = metric_registry();
        assert!(
            registry.len() >= 18,
            "the first M2 catalog must be substantive"
        );
        let mut ids = BTreeSet::new();
        for definition in registry {
            assert!(
                ids.insert(definition.id),
                "duplicate metric id {}",
                definition.id
            );
            assert!(!definition.formula.trim().is_empty());
            assert!(!definition.units.trim().is_empty());
            assert!(!definition.periodicity.trim().is_empty());
            assert!(!definition.annualization.trim().is_empty());
            assert!(!definition.degenerate_case.trim().is_empty());
        }
    }

    #[test]
    fn hand_ledger_reconciles_profit_drawdown_underwater_and_calendar_closes() {
        const DAY: i64 = 86_400_000_000_000;
        let input = report(
            vec![
                fill(0, 1, OrderSide::Buy, 1.0, 100.0, 0.0, 1.0),
                fill(1, DAY + 1, OrderSide::Sell, 1.0, 110.0, 10.0, 0.0),
                fill(2, DAY + 2, OrderSide::Buy, 1.0, 110.0, 0.0, 1.0),
                fill(3, 2 * DAY + 1, OrderSide::Sell, 1.0, 105.0, -5.0, 0.0),
            ],
            &[
                (0, 1_000.0),
                (DAY - 1, 1_008.0),
                (DAY + 1, 1_010.0),
                (2 * DAY - 1, 1_006.0),
                (2 * DAY + 1, 1_005.0),
            ],
        );
        let analysis = analyze_simulation(&input, &[], 1_000.0).expect("analysis");
        assert_eq!(
            analysis.metric("net_profit"),
            Some(&MetricValue::defined(5.0))
        );
        assert_eq!(
            analysis.metric("gross_profit"),
            Some(&MetricValue::defined(10.0))
        );
        assert_eq!(
            analysis.metric("gross_loss"),
            Some(&MetricValue::defined(-5.0))
        );
        assert_eq!(
            analysis.metric("profit_factor"),
            Some(&MetricValue::defined(2.0))
        );
        assert_eq!(
            analysis.metric("max_drawdown_absolute"),
            Some(&MetricValue::defined(5.0))
        );
        assert_eq!(analysis.underwater_curve.last().unwrap().drawdown, 5.0);
        assert_eq!(
            analysis
                .calendar
                .daily
                .iter()
                .map(|p| p.closing_equity)
                .collect::<Vec<_>>(),
            vec![1_008.0, 1_006.0, 1_005.0]
        );
        assert_eq!(
            analysis
                .calendar
                .daily
                .iter()
                .map(|p| p.change)
                .sum::<f64>(),
            5.0
        );
    }

    #[test]
    fn long_and_short_mae_mfe_exclude_pre_entry_and_post_exit_bars() {
        let input = report(
            vec![
                fill(0, 10, OrderSide::Buy, 2.0, 100.0, 0.0, 2.0),
                fill(1, 29, OrderSide::Sell, 2.0, 105.0, 10.0, 0.0),
                fill(2, 30, OrderSide::Sell, 1.0, 92.0, 0.0, -1.0),
                fill(3, 39, OrderSide::Buy, 1.0, 95.0, -3.0, 0.0),
            ],
            &[(0, 1_000.0), (39, 1_007.0)],
        );
        let analysis = analyze_simulation(&input, &streams(), 1_000.0).expect("analysis");
        assert_eq!(analysis.trades.len(), 2);
        let long = &analysis.trades[0];
        assert_eq!((long.mae, long.mfe), (20.0, 20.0));
        assert_eq!(long.capture_efficiency, Some(0.5));
        let short = &analysis.trades[1];
        assert_eq!((short.mae, short.mfe), (4.0, 4.0));
        assert_eq!(short.capture_efficiency, Some(-0.75));
        assert_eq!(
            long.r_multiple,
            MetricValue::undefined(UndefinedReason::MissingInitialRisk)
        );
    }

    #[test]
    fn degenerate_values_are_typed_and_uncertainty_is_honestly_deferred() {
        let analysis = analyze_simulation(
            &report(vec![], &[(0, 1_000.0), (DAY_NS + 1, 1_000.0)]),
            &[],
            1_000.0,
        )
        .expect("flat analysis");
        assert_eq!(
            analysis.metric("profit_factor"),
            Some(&MetricValue::undefined(UndefinedReason::NoLosingTrades))
        );
        assert_eq!(
            analysis.metric("sharpe_ratio"),
            Some(&MetricValue::undefined(UndefinedReason::ZeroVariance))
        );
        assert_eq!(
            analysis.uncertainty.headline_confidence_intervals,
            DeferredUncertainty::UnavailableUntilM4
        );
        let json = serde_json::to_string(&analysis).expect("serializes");
        assert!(!json.contains("999"));
        assert!(!json.contains("NaN"));
        assert!(!json.contains("inf"));
    }

    #[test]
    fn exposure_uses_the_union_of_overlapping_trade_intervals() {
        assert_eq!(super::union_duration(&[(0, 10), (2, 8)]), 10);
    }

    #[test]
    fn strictly_rising_equity_has_zero_stagnation() {
        let analysis = analyze_simulation(
            &report(vec![], &[(0, 1_000.0), (10, 1_001.0), (20, 1_002.0)]),
            &[],
            1_000.0,
        )
        .expect("analysis");
        assert_eq!(
            analysis.metric("longest_stagnation"),
            Some(&MetricValue::defined(0.0))
        );
    }

    /// Four flat round trips whose net PnLs are exactly `[-2, -1, 1, 2]`, so the
    /// whole distribution/exposure family can be derived by hand.
    fn symmetric_trade_ledger() -> SimulationReport {
        let mut fills = Vec::new();
        for (index, (exit_price, realized)) in
            [(98.0, -2.0), (99.0, -1.0), (101.0, 1.0), (102.0, 2.0)]
                .into_iter()
                .enumerate()
        {
            let order = index as u64 * 2;
            let entry_time = order as i64 + 1;
            fills.push(fill(
                order,
                entry_time,
                OrderSide::Buy,
                1.0,
                100.0,
                0.0,
                1.0,
            ));
            fills.push(fill(
                order + 1,
                entry_time + 1,
                OrderSide::Sell,
                1.0,
                exit_price,
                realized,
                0.0,
            ));
        }
        report(fills, &[(0, 1_000.0), (10 * DAY_NS, 1_000.0)])
    }

    fn defined(analysis: &super::StrategyAnalysis, id: &str) -> f64 {
        match analysis.metric(id) {
            Some(MetricValue::Defined { value }) => *value,
            other => panic!("{id} should be defined, got {other:?}"),
        }
    }

    #[test]
    fn distribution_and_exposure_metrics_match_a_hand_computed_symmetric_ledger() {
        let analysis =
            analyze_simulation(&symmetric_trade_ledger(), &[], 1_000.0).expect("analysis");
        let pnls: Vec<f64> = analysis.trades.iter().map(|trade| trade.net_pnl).collect();
        assert_eq!(pnls, vec![-2.0, -1.0, 1.0, 2.0]);

        // Return & profit: gross ±3 cancel exactly.
        assert_eq!(defined(&analysis, "net_profit"), 0.0);
        assert_eq!(defined(&analysis, "gross_profit"), 3.0);
        assert_eq!(defined(&analysis, "gross_loss"), -3.0);
        assert_eq!(defined(&analysis, "profit_factor"), 1.0);
        assert_eq!(defined(&analysis, "payoff_ratio"), 1.0);
        // Each entry notional is 1 unit at 100, so per-trade returns are
        // [-0.02, -0.01, 0.01, 0.02]. They cancel to zero in exact arithmetic;
        // none of the four is binary-exact, so a sub-ULP residue survives.
        assert!(defined(&analysis, "average_trade_percent").abs() < 1e-15);

        // Distribution: a symmetric sample has zero skew; excess kurtosis is
        // G2 = 3/((2)(1)) * (5*(8.5/6.25 - 3) + 6) = 1.5 * -2.2 = -3.3.
        assert_eq!(defined(&analysis, "trade_pnl_skewness"), 0.0);
        assert!((defined(&analysis, "trade_pnl_excess_kurtosis") + 3.3).abs() < 1e-12);
        assert_eq!(defined(&analysis, "max_consecutive_losses"), 2.0);
        assert_eq!(defined(&analysis, "max_consecutive_wins"), 2.0);
        // Top decile of 4 trades is ceil(4/10) = 1 trade: +2 of +3 gross profit.
        assert!((defined(&analysis, "top_decile_pnl_share") - 2.0 / 3.0).abs() < 1e-12);

        // Exposure: four sequential long round trips, never overlapping.
        assert_eq!(defined(&analysis, "long_trade_count"), 4.0);
        assert_eq!(defined(&analysis, "short_trade_count"), 0.0);
        assert_eq!(defined(&analysis, "max_concurrent_positions"), 1.0);
        // Traded notional 100+98+100+99+100+101+100+102 = 800 on 1,000 equity.
        assert_eq!(defined(&analysis, "turnover"), 0.8);
        // 4 trades over exactly 10 days: 4 * 365.2425 / 10.
        assert!((defined(&analysis, "trades_per_year") - 146.097).abs() < 1e-9);
    }

    #[test]
    fn a_flat_curve_reports_typed_undefined_rather_than_a_fabricated_ratio() {
        let analysis =
            analyze_simulation(&symmetric_trade_ledger(), &[], 1_000.0).expect("analysis");
        for (id, reason) in [
            ("return_on_max_drawdown", UndefinedReason::ZeroDenominator),
            ("average_drawdown", UndefinedReason::ZeroDenominator),
            ("calmar_ratio", UndefinedReason::ZeroDenominator),
            ("tail_ratio", UndefinedReason::ZeroDenominator),
            ("sharpe_ratio", UndefinedReason::ZeroVariance),
            ("sortino_ratio", UndefinedReason::ZeroVariance),
            (
                "equity_curve_r_squared",
                UndefinedReason::InsufficientObservations,
            ),
            ("k_ratio", UndefinedReason::InsufficientObservations),
            ("average_capture_efficiency", UndefinedReason::NoTrades),
        ] {
            assert_eq!(
                analysis.metric(id),
                Some(&MetricValue::undefined(reason)),
                "{id} must carry a typed reason"
            );
        }
        // Sterling still divides by the +0.10 floor, so it stays defined at zero
        // drawdown where Calmar cannot.
        assert_eq!(defined(&analysis, "sterling_ratio"), 0.0);
        // A percentile of an all-zero return series must not serialize as `-0.0`.
        assert_eq!(defined(&analysis, "daily_value_at_risk_95"), 0.0);
        assert!(
            !serde_json::to_string(&analysis)
                .expect("json")
                .contains("-0.0")
        );
    }

    /// Equity doubling by +25 % / -50 % steps, chosen so every daily return is
    /// exact in binary floating point.
    fn geometric_equity_ledger() -> SimulationReport {
        report(
            vec![],
            &[
                (0, 1_000.0),
                (DAY_NS - 1, 1_250.0),
                (2 * DAY_NS - 1, 625.0),
                (3 * DAY_NS - 1, 781.25),
                (4 * DAY_NS - 1, 390.625),
            ],
        )
    }

    #[test]
    fn tail_and_drawdown_metrics_match_hand_computed_exact_binary_returns() {
        let analysis =
            analyze_simulation(&geometric_equity_ledger(), &[], 1_000.0).expect("analysis");
        let returns: Vec<f64> = analysis
            .calendar
            .daily
            .iter()
            .map(|point| point.return_fraction.expect("return"))
            .collect();
        assert_eq!(returns, vec![0.25, -0.5, 0.25, -0.5]);

        // Lower-nearest-rank on 4 sorted returns [-0.5, -0.5, 0.25, 0.25]:
        // the 5th percentile lands on index 0 and the 95th on index 3.
        assert_eq!(defined(&analysis, "daily_value_at_risk_95"), 0.5);
        assert_eq!(
            defined(&analysis, "daily_conditional_value_at_risk_95"),
            0.5
        );
        assert_eq!(defined(&analysis, "worst_daily_return"), -0.5);
        assert_eq!(defined(&analysis, "tail_ratio"), 0.5);

        // Peak 1,250 at day 0; trough 390.625 at day 3.
        assert_eq!(defined(&analysis, "max_drawdown_absolute"), 859.375);
        assert_eq!(defined(&analysis, "max_drawdown_percent"), 0.6875);
        // Drawdowns strictly below their peak: 625, 468.75, 859.375.
        assert!((defined(&analysis, "average_drawdown") - 1_953.125 / 3.0).abs() < 1e-12);
        // sqrt(mean([0, 0, 0.25, 0.140625, 0.47265625])) = sqrt(0.17265625).
        assert!((defined(&analysis, "ulcer_index") - 0.17265625_f64.sqrt()).abs() < 1e-12);
        // The peak is never regained, so recovery time is 0 while stagnation
        // runs the full three days from the peak.
        assert_eq!(defined(&analysis, "max_time_to_recovery"), 0.0);
        assert_eq!(
            defined(&analysis, "longest_stagnation"),
            (3 * DAY_NS) as f64
        );
        assert_eq!(defined(&analysis, "return_on_max_drawdown"), 0.0);
    }

    #[test]
    fn every_calendar_granularity_reconciles_with_the_overall_equity_change() {
        let analysis =
            analyze_simulation(&geometric_equity_ledger(), &[], 1_000.0).expect("analysis");
        let calendar = &analysis.calendar;
        assert_eq!(calendar.daily.len(), 4);
        // 1970-01-01..04 is a single ISO week, month and year.
        assert_eq!(calendar.weekly.len(), 1);
        assert_eq!(calendar.monthly.len(), 1);
        assert_eq!(calendar.annual.len(), 1);

        let expected = 390.625 - 1_000.0;
        for (label, series) in [
            ("daily", &calendar.daily),
            ("weekly", &calendar.weekly),
            ("monthly", &calendar.monthly),
            ("annual", &calendar.annual),
        ] {
            assert_eq!(
                series.iter().map(|point| point.change).sum::<f64>(),
                expected,
                "{label} changes must telescope to the overall equity change"
            );
            assert_eq!(
                series.last().expect("close").closing_equity,
                390.625,
                "{label} must close on the final mark"
            );
        }
    }

    #[test]
    fn calendar_equity_reconciles_exactly_with_the_closed_trade_list() {
        // Two round trips, +10 then -5, marked to market on separate days.
        let ledger = report(
            vec![
                fill(0, 1, OrderSide::Buy, 1.0, 100.0, 0.0, 1.0),
                fill(1, DAY_NS + 1, OrderSide::Sell, 1.0, 110.0, 10.0, 0.0),
                fill(2, DAY_NS + 2, OrderSide::Buy, 1.0, 110.0, 0.0, 1.0),
                fill(3, 2 * DAY_NS + 1, OrderSide::Sell, 1.0, 105.0, -5.0, 0.0),
            ],
            &[
                (0, 1_000.0),
                (DAY_NS + 1, 1_010.0),
                (2 * DAY_NS + 1, 1_005.0),
            ],
        );
        let analysis = analyze_simulation(&ledger, &[], 1_000.0).expect("analysis");
        let traded: f64 = analysis.trades.iter().map(|trade| trade.net_pnl).sum();
        let banked: f64 = analysis.calendar.daily.iter().map(|p| p.change).sum();
        assert_eq!(traded, 5.0);
        assert_eq!(banked, traded);
        assert_eq!(defined(&analysis, "net_profit"), traded);
    }

    #[test]
    fn expectancy_equals_average_trade_because_scratch_trades_cancel() {
        // win_rate*mean(win) + loss_rate*mean(loss) reduces to (sum_w + sum_l)/N,
        // and scratches contribute zero to the numerator while still counting in
        // N. Pinning the identity keeps the two ids from silently diverging.
        let analysis =
            analyze_simulation(&symmetric_trade_ledger(), &[], 1_000.0).expect("analysis");
        assert_eq!(
            analysis.metric("expectancy"),
            analysis.metric("average_trade")
        );
    }

    #[test]
    fn a_straight_line_equity_curve_fits_perfectly() {
        let ledger = report(
            vec![],
            &[
                (0, 1_000.0),
                (DAY_NS, 1_100.0),
                (2 * DAY_NS, 1_200.0),
                (3 * DAY_NS, 1_300.0),
            ],
        );
        let analysis = analyze_simulation(&ledger, &[], 1_000.0).expect("analysis");
        assert_eq!(defined(&analysis, "equity_curve_r_squared"), 1.0);
        // A zero-residual fit has no slope standard error to divide by.
        assert_eq!(
            analysis.metric("k_ratio"),
            Some(&MetricValue::undefined(UndefinedReason::ZeroVariance))
        );
    }

    #[test]
    fn diagnostics_report_the_ledger_cost_breakdown_verbatim() {
        let mut buy = fill(0, 1, OrderSide::Buy, 1.0, 100.0, 0.0, 1.0);
        buy.commission = 1.0;
        buy.spread_cost = 0.5;
        buy.slippage_cost = 0.25;
        let mut sell = fill(1, 2, OrderSide::Sell, 1.0, 110.0, 10.0, 0.0);
        sell.commission = 1.0;
        sell.spread_cost = 0.5;
        sell.slippage_cost = 0.25;
        let mut ledger = report(vec![buy, sell], &[(0, 1_000.0), (10, 1_006.5)]);
        ledger.final_realized_pnl = 6.5;
        ledger.total_commission = 2.0;

        let analysis = analyze_simulation(&ledger, &[], 1_000.0).expect("analysis");
        let diagnostics = &analysis.diagnostics;
        assert_eq!(diagnostics.commission_cost, 2.0);
        assert_eq!(diagnostics.spread_cost, 1.0);
        assert_eq!(diagnostics.slippage_cost, 0.5);
        // 6.5 realized + 3.5 of costs = 10.0 gross; costs are 35 % of gross.
        assert_eq!(diagnostics.gross_pnl_before_costs, 10.0);
        assert_eq!(
            diagnostics.cost_share_of_gross_pnl,
            MetricValue::defined(0.35)
        );
        assert_eq!(diagnostics.fill_count, 2);
        assert_eq!(diagnostics.open_position_count, 0);
    }

    #[test]
    fn every_registered_metric_is_computed_rather_than_silently_defaulted() {
        // `analyze_simulation` fills registry ids it cannot find with a generic
        // undefined value. That fallback must never fire in practice, or a typo
        // in a metric id would masquerade as "not enough data".
        let ledger = geometric_equity_ledger();
        let computed = super::compute_metrics(
            &[],
            &super::underwater(&ledger.equity_curve),
            &super::resample_calendar(&ledger.equity_curve, 1_000.0),
            &ledger,
            1_000.0,
        );
        let registered: BTreeSet<&str> = metric_registry()
            .iter()
            .map(|definition| definition.id)
            .collect();
        let produced: BTreeSet<&str> = computed.keys().copied().collect();
        assert_eq!(
            produced, registered,
            "compute_metrics and the registry must agree exactly"
        );
    }

    #[test]
    fn registry_definitions_are_pinned_to_the_metrics_schema_version() {
        // Any edit to an id, formula, unit, periodicity, annualization rule or
        // degenerate case changes the meaning of a published number, so it must
        // come with a METRICS_SCHEMA_VERSION bump. This digest forces that.
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        for definition in metric_registry() {
            for field in [
                definition.id,
                definition.formula,
                definition.units,
                definition.periodicity,
                definition.annualization,
                definition.degenerate_case,
            ] {
                hasher.update((field.len() as u64).to_be_bytes());
                hasher.update(field.as_bytes());
            }
        }
        let digest: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            (METRICS_SCHEMA_VERSION, digest.as_str()),
            ("strategy-metrics/v1", REGISTRY_DIGEST_V1),
            "metric definitions changed without a metrics version bump"
        );
    }

    const REGISTRY_DIGEST_V1: &str =
        "71c52febbc41195ffec953aef0cccf5924ef62f3c268d3c763621184877f50da";
}
