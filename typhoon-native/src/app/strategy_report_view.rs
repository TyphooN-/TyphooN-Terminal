//! Bounded, precomputed native presentation for an identity-bound strategy run.
//!
//! Construction is deliberately separate from egui rendering: digest verification,
//! ledger grouping, timestamp indexing, curve preparation, and metric cloning happen
//! once when a completed run is delivered. Repaint code only indexes these vectors.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use typhoon_chart_ui::drawing::{PositionLine, TradeMarker, TradeOverlay};
use typhoon_engine::core::strategy_metrics::{
    MetricResult, MetricValue, StrategyAnalysis, TradeDirection, UndefinedReason,
};
use typhoon_engine::core::strategy_report::StrategyReportArtifact;
use typhoon_engine::core::strategy_simulator::{
    FillRecord, OrderKind, OrderSide, SimulationReport, SymbolId,
};

pub(crate) const MAX_PREPARED_REPORT_ITEMS: usize = 250_000;
const MAX_SIMULATION_REPORT_JSON_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug)]
pub(crate) enum StrategyResultViewError {
    Verification(String),
    TooManyItems {
        field: &'static str,
        limit: usize,
        found: usize,
    },
    UnknownSymbol(SymbolId),
    EmptyChartTimeline,
    UnorderedChartTimeline,
}

impl fmt::Display for StrategyResultViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("cannot prepare verified strategy result: ")?;
        match self {
            Self::Verification(error) => write!(formatter, "identity verification failed: {error}"),
            Self::TooManyItems {
                field,
                limit,
                found,
            } => {
                write!(formatter, "{field} has {found} items (limit {limit})")
            }
            Self::UnknownSymbol(symbol) => write!(formatter, "unknown symbol index {}", symbol.0),
            Self::EmptyChartTimeline => formatter.write_str("chart timeline is empty"),
            Self::UnorderedChartTimeline => {
                formatter.write_str("chart timeline is not strictly ordered")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TradeReplayVisibility {
    Future,
    Open,
    Closed,
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedTrade {
    pub(crate) trade_id: u64,
    pub(crate) direction: TradeDirection,
    pub(crate) entry_bar: usize,
    pub(crate) exit_bar: usize,
    pub(crate) entry_time_ns: i64,
    pub(crate) exit_time_ns: i64,
    pub(crate) quantity: f64,
    pub(crate) entry_price: f64,
    pub(crate) exit_price: f64,
    pub(crate) net_pnl: f64,
    pub(crate) mae: f64,
    pub(crate) mfe: f64,
}

impl PreparedTrade {
    pub(crate) fn visibility_at_bar(&self, visible_bar: usize) -> TradeReplayVisibility {
        if visible_bar < self.entry_bar {
            TradeReplayVisibility::Future
        } else if visible_bar < self.exit_bar {
            TradeReplayVisibility::Open
        } else {
            TradeReplayVisibility::Closed
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PreparedCurvePoint {
    pub(crate) time_ns: i64,
    pub(crate) value: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct StrategyResultView {
    pub(crate) run_id: String,
    pub(crate) report_id: String,
    pub(crate) metrics_version: String,
    pub(crate) symbol: String,
    pub(crate) symbol_id: SymbolId,
    pub(crate) chart_bar_count: usize,
    pub(crate) overlay: TradeOverlay,
    pub(crate) trades: Vec<PreparedTrade>,
    pub(crate) equity: Vec<PreparedCurvePoint>,
    pub(crate) drawdown: Vec<PreparedCurvePoint>,
    pub(crate) metrics: Vec<MetricResult>,
    pub(crate) diagnostics: typhoon_engine::core::strategy_metrics::Diagnostics,
}

impl StrategyResultView {
    /// Prepare one chart-symbol view. Call this on the run/result worker, never
    /// from an egui paint callback.
    pub(crate) fn prepare(
        artifact: &StrategyReportArtifact,
        report: &SimulationReport,
        symbol_id: SymbolId,
        chart_bar_times_ms: &[i64],
    ) -> Result<Self, StrategyResultViewError> {
        artifact
            .verify_simulation_report(report)
            .map_err(|error| StrategyResultViewError::Verification(error.to_string()))?;
        validate_chart_timeline(chart_bar_times_ms)?;
        check_report_bounds(report)?;
        let analysis = artifact.analysis();
        check_analysis_bounds(analysis)?;
        let symbol = report
            .symbols
            .get(symbol_id.0)
            .cloned()
            .ok_or(StrategyResultViewError::UnknownSymbol(symbol_id))?;

        let mut overlay = TradeOverlay {
            markers: group_fill_markers(
                &report.fills,
                symbol_id,
                chart_bar_times_ms,
                artifact.report_id(),
            )?,
            ..TradeOverlay::default()
        };
        overlay.position_lines =
            protective_lines(report, symbol_id, chart_bar_times_ms, artifact.report_id());

        let mut trades: Vec<_> = analysis
            .trades
            .iter()
            .filter(|trade| trade.symbol == symbol_id)
            .map(|trade| PreparedTrade {
                trade_id: trade.trade_id,
                direction: trade.direction,
                entry_bar: bar_index_for_time_ns(chart_bar_times_ms, trade.entry_time_ns),
                exit_bar: bar_index_for_time_ns(chart_bar_times_ms, trade.exit_time_ns),
                entry_time_ns: trade.entry_time_ns,
                exit_time_ns: trade.exit_time_ns,
                quantity: trade.quantity,
                entry_price: trade.average_entry_price,
                exit_price: trade.average_exit_price,
                net_pnl: trade.net_pnl,
                mae: trade.mae,
                mfe: trade.mfe,
            })
            .collect();
        trades.sort_by(|left, right| {
            left.entry_bar
                .cmp(&right.entry_bar)
                .then_with(|| left.exit_bar.cmp(&right.exit_bar))
                .then_with(|| left.trade_id.cmp(&right.trade_id))
        });
        let equity = report
            .equity_curve
            .iter()
            .map(|point| PreparedCurvePoint {
                time_ns: point.time_ns,
                value: point.equity,
            })
            .collect();
        let drawdown = analysis
            .underwater_curve
            .iter()
            .map(|point| PreparedCurvePoint {
                time_ns: point.time_ns,
                value: -point.drawdown,
            })
            .collect();

        Ok(Self {
            run_id: artifact.run_id().to_string(),
            report_id: artifact.report_id().to_string(),
            metrics_version: artifact.metrics_version().to_string(),
            symbol,
            symbol_id,
            chart_bar_count: chart_bar_times_ms.len(),
            overlay,
            trades,
            equity,
            drawdown,
            metrics: analysis.metrics.clone(),
            diagnostics: analysis.diagnostics.clone(),
        })
    }

    pub(crate) fn metric(&self, id: &str) -> Option<&MetricValue> {
        self.metrics
            .iter()
            .find(|metric| metric.id == id)
            .map(|metric| &metric.value)
    }
}

/// Read, identify, verify, and prepare a report-artifact/simulation-report pair.
/// Filesystem I/O, JSON parsing, digest verification, and ledger indexing all
/// happen here, so callers must run this on a worker thread.
pub(crate) fn load_prepared_pair(
    paths: [PathBuf; 2],
    chart_symbol: &str,
    chart_bar_times_ms: &[i64],
) -> Result<StrategyResultView, String> {
    let first = read_bounded(&paths[0], MAX_SIMULATION_REPORT_JSON_BYTES)?;
    let second = read_bounded(&paths[1], MAX_SIMULATION_REPORT_JSON_BYTES)?;
    let first_artifact = StrategyReportArtifact::from_json_slice(&first).ok();
    let second_artifact = StrategyReportArtifact::from_json_slice(&second).ok();
    let (artifact, report_bytes) = match (first_artifact, second_artifact) {
        (Some(artifact), None) => (artifact, second.as_slice()),
        (None, Some(artifact)) => (artifact, first.as_slice()),
        (Some(_), Some(_)) => {
            return Err("select one report artifact and one simulation report".into());
        }
        (None, None) => {
            return Err("neither selected file is a valid strategy report artifact".into());
        }
    };
    let report: SimulationReport = serde_json::from_slice(report_bytes)
        .map_err(|error| format!("invalid simulation report JSON: {error}"))?;
    let symbol_id = report
        .symbols
        .iter()
        .position(|symbol| symbols_match(symbol, chart_symbol))
        .map(SymbolId)
        .ok_or_else(|| format!("report has no symbol matching active chart {chart_symbol}"))?;
    StrategyResultView::prepare(&artifact, &report, symbol_id, chart_bar_times_ms)
        .map_err(|error| error.to_string())
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let len = std::fs::metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
        .len();
    if len > limit as u64 {
        return Err(format!(
            "{} exceeds the {limit}-byte load limit",
            path.display()
        ));
    }
    std::fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn symbols_match(left: &str, right: &str) -> bool {
    fn canonical(value: &str) -> String {
        value
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .flat_map(char::to_uppercase)
            .collect()
    }
    canonical(left) == canonical(right)
}

fn check_analysis_bounds(analysis: &StrategyAnalysis) -> Result<(), StrategyResultViewError> {
    for (field, len) in [
        ("analysis.metrics", analysis.metrics.len()),
        ("analysis.trades", analysis.trades.len()),
        ("analysis.underwater_curve", analysis.underwater_curve.len()),
        ("analysis.calendar.daily", analysis.calendar.daily.len()),
    ] {
        check_bound(field, len)?;
    }
    Ok(())
}

fn check_report_bounds(report: &SimulationReport) -> Result<(), StrategyResultViewError> {
    for (field, len) in [
        ("report.symbols", report.symbols.len()),
        ("report.events", report.events.len()),
        ("report.fills", report.fills.len()),
        ("report.rejections", report.rejections.len()),
        ("report.cancellations", report.cancellations.len()),
        ("report.pending_orders", report.pending_orders.len()),
        ("report.positions", report.positions.len()),
        ("report.equity_curve", report.equity_curve.len()),
        ("report.financing_charges", report.financing_charges.len()),
        ("report.corporate_actions", report.corporate_actions.len()),
    ] {
        check_bound(field, len)?;
    }
    Ok(())
}

fn validate_chart_timeline(chart_bar_times_ms: &[i64]) -> Result<(), StrategyResultViewError> {
    if chart_bar_times_ms.is_empty() {
        return Err(StrategyResultViewError::EmptyChartTimeline);
    }
    if chart_bar_times_ms.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(StrategyResultViewError::UnorderedChartTimeline);
    }
    Ok(())
}

fn check_bound(field: &'static str, found: usize) -> Result<(), StrategyResultViewError> {
    if found > MAX_PREPARED_REPORT_ITEMS {
        Err(StrategyResultViewError::TooManyItems {
            field,
            limit: MAX_PREPARED_REPORT_ITEMS,
            found,
        })
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum MarkerSemantic {
    Entry,
    PartialEntry,
    Exit,
    PartialExit,
    ExitEntry,
}

impl MarkerSemantic {
    const fn label(self) -> &'static str {
        match self {
            Self::Entry => "ENTRY",
            Self::PartialEntry => "PARTIAL ENTRY",
            Self::Exit => "EXIT",
            Self::PartialExit => "PARTIAL EXIT",
            Self::ExitEntry => "EXIT/ENTRY",
        }
    }
}

fn fill_semantic(fill: &FillRecord) -> MarkerSemantic {
    let signed_quantity = match fill.side {
        OrderSide::Buy => fill.quantity,
        OrderSide::Sell => -fill.quantity,
    };
    let before = fill.position_units_after - signed_quantity;
    let after = fill.position_units_after;
    if before != 0.0 && after != 0.0 && before.signum() != after.signum() {
        MarkerSemantic::ExitEntry
    } else if after.abs() > before.abs() {
        if fill.remaining_quantity > 0.0 {
            MarkerSemantic::PartialEntry
        } else {
            MarkerSemantic::Entry
        }
    } else if after != 0.0 {
        MarkerSemantic::PartialExit
    } else {
        MarkerSemantic::Exit
    }
}

pub(crate) fn group_fill_markers(
    fills: &[FillRecord],
    symbol: SymbolId,
    chart_bar_times_ms: &[i64],
    report_id: &str,
) -> Result<Vec<TradeMarker>, StrategyResultViewError> {
    check_bound("fills", fills.len())?;
    validate_chart_timeline(chart_bar_times_ms)?;
    let mut grouped: HashMap<(usize, bool, u64, MarkerSemantic), (f64, u32, u64)> = HashMap::new();
    for fill in fills.iter().filter(|fill| fill.symbol == symbol) {
        let bar_idx = bar_index_for_time_ns(chart_bar_times_ms, fill.time_ns);
        let semantic = fill_semantic(fill);
        grouped
            .entry((
                bar_idx,
                fill.side == OrderSide::Buy,
                fill.fill_price.to_bits(),
                semantic,
            ))
            .and_modify(|entry| {
                entry.0 += fill.quantity;
                entry.1 = entry.1.saturating_add(1);
                entry.2 = entry.2.min(fill.sequence);
            })
            .or_insert((fill.quantity, 1, fill.sequence));
    }
    let identity = short_identity(report_id);
    let mut markers: Vec<_> = grouped
        .into_iter()
        .map(
            |((bar_idx, is_buy, price_bits, semantic), (volume, count, first_sequence))| {
                (
                    first_sequence,
                    TradeMarker {
                        bar_idx,
                        price: f64::from_bits(price_bits),
                        volume,
                        is_buy,
                        count,
                        ticker: format!("SIM {identity} {}", semantic.label()),
                    },
                )
            },
        )
        .collect();
    markers.sort_by(|left, right| {
        left.1
            .bar_idx
            .cmp(&right.1.bar_idx)
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.1.ticker.cmp(&right.1.ticker))
            .then_with(|| left.1.price.total_cmp(&right.1.price))
            .then_with(|| left.1.is_buy.cmp(&right.1.is_buy))
            .then_with(|| left.1.volume.total_cmp(&right.1.volume))
            .then_with(|| left.1.count.cmp(&right.1.count))
    });
    Ok(markers.into_iter().map(|(_, marker)| marker).collect())
}

fn protective_lines(
    report: &SimulationReport,
    symbol: SymbolId,
    chart_bar_times_ms: &[i64],
    report_id: &str,
) -> Vec<PositionLine> {
    let final_units = report
        .positions
        .iter()
        .find(|position| position.symbol == symbol)
        .map_or(0.0, |position| position.units);
    if final_units == 0.0 {
        return Vec::new();
    }
    let protective_side = if final_units > 0.0 {
        OrderSide::Sell
    } else {
        OrderSide::Buy
    };
    let identity = short_identity(report_id);
    let mut lines = Vec::new();
    for order in report
        .pending_orders
        .iter()
        .filter(|order| order.symbol == symbol && order.side == protective_side)
    {
        let start_bar = bar_index_for_time_ns(chart_bar_times_ms, order.submitted_time_ns);
        let (price, line_type, role) = match order.kind {
            OrderKind::Stop { stop_price } => (stop_price, 1, "SL"),
            OrderKind::Limit { limit_price } => (limit_price, 2, "TP"),
            OrderKind::StopLimit { stop_price, .. } => (stop_price, 1, "SL"),
            OrderKind::Market | OrderKind::MarketOnClose => continue,
        };
        lines.push(PositionLine {
            price,
            volume: (order.quantity - order.filled_quantity).max(0.0),
            is_buy: final_units > 0.0,
            line_type,
            start_bar,
            end_bar: usize::MAX,
            label: Some(format!("SIM {identity} {role}")),
        });
    }
    lines.sort_by(|left, right| {
        left.start_bar
            .cmp(&right.start_bar)
            .then_with(|| left.line_type.cmp(&right.line_type))
            .then_with(|| left.price.total_cmp(&right.price))
            .then_with(|| left.is_buy.cmp(&right.is_buy))
            .then_with(|| left.volume.total_cmp(&right.volume))
            .then_with(|| left.label.cmp(&right.label))
    });
    lines
}

pub(crate) fn bar_index_for_time_ns(chart_bar_times_ms: &[i64], time_ns: i64) -> usize {
    let time_ms = time_ns.div_euclid(1_000_000);
    chart_bar_times_ms
        .partition_point(|bar_time| *bar_time <= time_ms)
        .saturating_sub(1)
        .min(chart_bar_times_ms.len().saturating_sub(1))
}

pub(crate) fn clamp_selected_trade(selected: Option<usize>, len: usize) -> Option<usize> {
    (len > 0).then(|| selected.unwrap_or(0).min(len - 1))
}

pub(crate) fn next_trade_index(selected: Option<usize>, len: usize) -> Option<usize> {
    (len > 0).then(|| (selected.unwrap_or(0) + 1) % len)
}

pub(crate) fn previous_trade_index(selected: Option<usize>, len: usize) -> Option<usize> {
    (len > 0).then(|| selected.unwrap_or(0).checked_sub(1).unwrap_or(len - 1))
}

pub(crate) fn clamp_replay_bar(bar: usize, chart_bar_count: usize) -> usize {
    bar.clamp(1, chart_bar_count.max(1))
}

pub(crate) const fn reset_replay_bar(_chart_bar_count: usize) -> usize {
    1
}

pub(crate) fn step_replay_bar(bar: usize, chart_bar_count: usize) -> usize {
    bar.saturating_add(1).min(chart_bar_count.max(1))
}

pub(crate) fn clamp_replay_speed(speed: f32) -> f32 {
    speed.clamp(0.5, 60.0)
}

pub(crate) fn revealed_trade_count(trades: &[PreparedTrade], replay_bar: usize) -> usize {
    trades.partition_point(|trade| trade.entry_bar < replay_bar)
}

pub(crate) fn format_metric_value(id: &str, value: &MetricValue) -> String {
    match value {
        MetricValue::Defined { value } => {
            if id.contains("percent") || matches!(id, "total_return" | "cagr" | "time_in_market") {
                format!("{:.2}%", value * 100.0)
            } else if id.contains("duration")
                || id.contains("stagnation")
                || id.contains("recovery")
            {
                format_duration_ns(*value)
            } else if id.contains("profit")
                || id.contains("drawdown_absolute")
                || id.contains("trade")
                || id.contains("mae")
                || id.contains("mfe")
            {
                format!("{value:.2}")
            } else {
                format!("{value:.4}")
            }
        }
        MetricValue::Undefined { reason } => {
            format!("Undefined ({})", undefined_reason_label(*reason))
        }
    }
}

fn format_duration_ns(value: f64) -> String {
    let seconds = (value / 1_000_000_000.0).max(0.0);
    if seconds >= 86_400.0 {
        format!("{:.1}d", seconds / 86_400.0)
    } else if seconds >= 3_600.0 {
        format!("{:.1}h", seconds / 3_600.0)
    } else {
        format!("{seconds:.0}s")
    }
}

const fn undefined_reason_label(reason: UndefinedReason) -> &'static str {
    match reason {
        UndefinedReason::NoTrades => "no trades",
        UndefinedReason::NoWinningTrades => "no winning trades",
        UndefinedReason::NoLosingTrades => "no losing trades",
        UndefinedReason::ZeroDenominator => "zero denominator",
        UndefinedReason::ZeroVariance => "zero variance",
        UndefinedReason::MissingInitialRisk => "missing initial risk",
        UndefinedReason::InsufficientObservations => "insufficient observations",
        UndefinedReason::ArithmeticOverflow => "arithmetic overflow",
    }
}

fn short_identity(identity: &str) -> &str {
    identity.get(..8).unwrap_or(identity)
}

pub(crate) fn render_prepared_result(
    ui: &mut egui::Ui,
    view: &StrategyResultView,
    selected: &mut Option<usize>,
    replay_active: &mut bool,
    replay_bar_idx: &mut usize,
    replay_playing: &mut bool,
    replay_speed: &mut f32,
) {
    ui.separator();
    ui.heading("Verified Strategy Report");
    ui.horizontal_wrapped(|ui| {
        ui.strong(&view.symbol);
        ui.label(format!("symbol #{}", view.symbol_id.0));
        ui.label(format!("run {}", short_identity(&view.run_id)));
        ui.label(format!("report {}", short_identity(&view.report_id)));
        ui.label(format!("metrics {}", view.metrics_version));
    });
    ui.label(format!(
        "{} trades · {} fills · {} rejected · {} cancelled",
        view.trades.len(),
        view.diagnostics.fill_count,
        view.diagnostics.rejected_order_count,
        view.diagnostics.cancelled_order_count
    ));
    if let (Some(first), Some(last)) = (view.equity.first(), view.equity.last()) {
        ui.label(format!(
            "Equity {:.2} → {:.2} across {:.1}h",
            first.value,
            last.value,
            (last.time_ns - first.time_ns).max(0) as f64 / 3_600_000_000_000.0
        ));
    }
    if let Some(worst) = view
        .drawdown
        .iter()
        .min_by(|left, right| left.value.total_cmp(&right.value))
    {
        ui.label(format!(
            "Worst drawdown {:.2} at {} ns",
            worst.value, worst.time_ns
        ));
    }

    ui.horizontal(|ui| {
        if ui
            .selectable_label(!*replay_active, "Full result")
            .clicked()
        {
            *replay_active = false;
            *replay_playing = false;
        }
        if ui.selectable_label(*replay_active, "Replay").clicked() {
            *replay_active = true;
            *replay_playing = false;
            *replay_bar_idx = clamp_replay_bar(*replay_bar_idx, view.chart_bar_count);
        }
        if *replay_active {
            if ui
                .button(if *replay_playing { "Pause" } else { "Play" })
                .clicked()
            {
                *replay_playing = !*replay_playing;
            }
            if ui.button("Step").clicked() {
                *replay_playing = false;
                *replay_bar_idx = step_replay_bar(*replay_bar_idx, view.chart_bar_count);
            }
            if ui.button("Reset").clicked() {
                *replay_playing = false;
                *replay_bar_idx = reset_replay_bar(view.chart_bar_count);
            }
            *replay_speed = clamp_replay_speed(*replay_speed);
            ui.add(egui::Slider::new(replay_speed, 0.5..=60.0).text("bars/s"));
            ui.add(
                egui::Slider::new(replay_bar_idx, 1..=view.chart_bar_count.max(1))
                    .text("visible bars"),
            );
        }
    });

    if *replay_active {
        ui.label(
            egui::RichText::new("Full-run metrics and future trade details are hidden in replay.")
                .color(egui::Color32::from_rgb(255, 200, 50)),
        );
    } else {
        egui::Grid::new("verified_strategy_metrics")
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                for id in [
                    "total_return",
                    "max_drawdown_percent",
                    "profit_factor",
                    "sharpe_ratio",
                    "expectancy",
                    "time_in_market",
                ] {
                    if let Some(value) = view.metric(id) {
                        ui.label(id.replace('_', " "));
                        ui.label(format_metric_value(id, value));
                        ui.end_row();
                    }
                }
            });
    }

    let revealed = if *replay_active {
        revealed_trade_count(&view.trades, *replay_bar_idx)
    } else {
        view.trades.len()
    };
    *selected = clamp_selected_trade(*selected, revealed);
    ui.horizontal(|ui| {
        ui.strong(format!("Trade inspector ({revealed} revealed)"));
        if ui
            .add_enabled(revealed > 0, egui::Button::new("Previous"))
            .clicked()
        {
            *selected = previous_trade_index(*selected, revealed);
        }
        if ui
            .add_enabled(revealed > 0, egui::Button::new("Next"))
            .clicked()
        {
            *selected = next_trade_index(*selected, revealed);
        }
    });
    let Some(trade) = selected.and_then(|index| view.trades.get(index)) else {
        ui.label("No trade is visible at this replay position.");
        return;
    };
    let visible_bar = if *replay_active {
        replay_bar_idx.saturating_sub(1)
    } else {
        usize::MAX
    };
    ui.label(format!(
        "#{} {:?} · qty {:.4} · entry {:.6} at {} ns",
        trade.trade_id, trade.direction, trade.quantity, trade.entry_price, trade.entry_time_ns
    ));
    match trade.visibility_at_bar(visible_bar) {
        TradeReplayVisibility::Future => {
            ui.label("Trade has not entered yet.");
        }
        TradeReplayVisibility::Open => {
            ui.label(format!(
                "Open · MAE {:.2} · MFE {:.2}",
                trade.mae, trade.mfe
            ));
        }
        TradeReplayVisibility::Closed => {
            let color = if trade.net_pnl >= 0.0 {
                egui::Color32::from_rgb(46, 204, 113)
            } else {
                egui::Color32::from_rgb(231, 76, 60)
            };
            ui.label(
                egui::RichText::new(format!(
                    "Exit {:.6} at {} ns · P&L {:.2} · MAE {:.2} · MFE {:.2}",
                    trade.exit_price, trade.exit_time_ns, trade.net_pnl, trade.mae, trade.mfe
                ))
                .color(color),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typhoon_engine::core::strategy_metrics::{MetricValue, TradeDirection, UndefinedReason};
    use typhoon_engine::core::strategy_simulator::{
        ClientOrderId, FillRecord, OrderSide, SymbolId,
    };

    fn fill(
        id: u64,
        time_ns: i64,
        side: OrderSide,
        quantity: f64,
        remaining: f64,
        price: f64,
        position_after: f64,
    ) -> FillRecord {
        FillRecord {
            order_id: ClientOrderId(id),
            time_ns,
            sequence: id,
            symbol: SymbolId(0),
            side,
            quantity,
            remaining_quantity: remaining,
            reference_price: price,
            quoted_price: price,
            fill_price: price,
            spread_cost: 0.0,
            slippage_cost: 0.0,
            commission: 0.0,
            conversion_rate: 1.0,
            conversion_cost: 0.0,
            realized_pnl: 0.0,
            cash_after: 1_000.0,
            position_units_after: position_after,
            avg_entry_after: price,
        }
    }

    #[test]
    fn fill_markers_are_grouped_sorted_and_keep_partial_fill_semantics() {
        let bars = [1_000_i64, 2_000, 3_000, 4_000];
        let fills = vec![
            fill(1, 2_100_000_000, OrderSide::Buy, 2.0, 3.0, 10.0, 2.0),
            fill(2, 2_200_000_000, OrderSide::Buy, 3.0, 0.0, 10.0, 5.0),
            fill(3, 3_100_000_000, OrderSide::Sell, 2.0, 0.0, 11.0, 3.0),
        ];

        let markers = group_fill_markers(&fills, SymbolId(0), &bars, "0123456789abcdef")
            .expect("bounded grouping");

        assert_eq!(markers.len(), 3);
        assert_eq!(markers[0].bar_idx, 1);
        assert!(markers[0].ticker.contains("PARTIAL ENTRY"));
        assert_eq!(markers[1].bar_idx, 1);
        assert!(markers[1].ticker.contains("ENTRY"));
        assert_eq!(markers[2].bar_idx, 2);
        assert!(markers[2].ticker.contains("PARTIAL EXIT"));
        assert!(
            markers
                .iter()
                .all(|marker| marker.ticker.contains("01234567"))
        );
    }

    #[test]
    fn fill_marker_preparation_rejects_unbounded_inputs() {
        let fills = vec![
            fill(1, 1_000_000_000, OrderSide::Buy, 1.0, 0.0, 10.0, 1.0);
            MAX_PREPARED_REPORT_ITEMS + 1
        ];
        let error = group_fill_markers(&fills, SymbolId(0), &[1_000], "report")
            .expect_err("input cap is enforced before grouping");
        assert!(matches!(
            error,
            StrategyResultViewError::TooManyItems { .. }
        ));
    }

    #[test]
    fn chart_timeline_must_be_strictly_ordered() {
        assert!(validate_chart_timeline(&[1_000, 2_000, 3_000]).is_ok());
        assert!(matches!(
            validate_chart_timeline(&[1_000, 1_000]),
            Err(StrategyResultViewError::UnorderedChartTimeline)
        ));
        assert!(matches!(
            validate_chart_timeline(&[2_000, 1_000]),
            Err(StrategyResultViewError::UnorderedChartTimeline)
        ));
    }

    #[test]
    fn selected_trade_index_is_clamped_in_constant_time() {
        assert_eq!(clamp_selected_trade(None, 0), None);
        assert_eq!(clamp_selected_trade(Some(99), 3), Some(2));
        assert_eq!(next_trade_index(Some(2), 3), Some(0));
        assert_eq!(previous_trade_index(Some(0), 3), Some(2));
    }

    #[test]
    fn replay_controls_are_bounded_by_the_prepared_chart_timeline() {
        assert_eq!(reset_replay_bar(0), 1);
        assert_eq!(reset_replay_bar(20), 1);
        assert_eq!(step_replay_bar(19, 20), 20);
        assert_eq!(step_replay_bar(20, 20), 20);
        assert_eq!(clamp_replay_bar(99, 20), 20);
        assert_eq!(clamp_replay_bar(0, 20), 1);
        assert_eq!(clamp_replay_speed(0.1), 0.5);
        assert_eq!(clamp_replay_speed(100.0), 60.0);
    }

    #[test]
    fn revealed_trade_count_uses_stable_entry_order_and_excludes_future_entries() {
        let trades = vec![
            prepared_trade(1, 1, 3),
            prepared_trade(2, 4, 6),
            prepared_trade(3, 4, 8),
        ];
        assert_eq!(revealed_trade_count(&trades, 1), 0);
        assert_eq!(revealed_trade_count(&trades, 2), 1);
        assert_eq!(revealed_trade_count(&trades, 4), 1);
        assert_eq!(revealed_trade_count(&trades, 5), 3);
    }

    #[test]
    fn replay_bounds_hide_future_trade_details() {
        let trade = PreparedTrade {
            trade_id: 7,
            direction: TradeDirection::Long,
            entry_bar: 2,
            exit_bar: 6,
            entry_time_ns: 2_000_000_000,
            exit_time_ns: 6_000_000_000,
            quantity: 1.0,
            entry_price: 10.0,
            exit_price: 12.0,
            net_pnl: 2.0,
            mae: 1.0,
            mfe: 3.0,
        };

        assert_eq!(trade.visibility_at_bar(1), TradeReplayVisibility::Future);
        assert_eq!(trade.visibility_at_bar(4), TradeReplayVisibility::Open);
        assert_eq!(trade.visibility_at_bar(6), TradeReplayVisibility::Closed);
    }

    fn prepared_trade(trade_id: u64, entry_bar: usize, exit_bar: usize) -> PreparedTrade {
        PreparedTrade {
            trade_id,
            direction: TradeDirection::Long,
            entry_bar,
            exit_bar,
            entry_time_ns: entry_bar as i64 * 1_000_000_000,
            exit_time_ns: exit_bar as i64 * 1_000_000_000,
            quantity: 1.0,
            entry_price: 10.0,
            exit_price: 11.0,
            net_pnl: 1.0,
            mae: 0.5,
            mfe: 1.5,
        }
    }

    #[test]
    fn metric_formatting_is_typed_and_unit_aware() {
        assert_eq!(
            format_metric_value("max_drawdown_percent", &MetricValue::defined(0.1234)),
            "12.34%"
        );
        assert_eq!(
            format_metric_value(
                "profit_factor",
                &MetricValue::undefined(UndefinedReason::NoLosingTrades)
            ),
            "Undefined (no losing trades)"
        );
    }
}
