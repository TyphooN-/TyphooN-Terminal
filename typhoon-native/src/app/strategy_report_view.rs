//! Bounded, precomputed native presentation for an identity-bound strategy run.
//!
//! Construction is deliberately separate from egui rendering: digest verification,
//! ledger grouping, timestamp indexing, curve preparation, and metric cloning happen
//! once when a completed run is delivered. Repaint code only indexes these vectors.

use std::collections::HashMap;
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

use typhoon_chart_ui::drawing::{PositionLine, TradeMarker, TradeOverlay};
use typhoon_engine::core::strategy_intervention::{
    Intervention, InterventionAction, InterventionLog, MAX_INTERVENTION_NOTE_BYTES,
    MAX_INTERVENTIONS,
};
use typhoon_engine::core::strategy_ir::RepaintAcknowledgement;
use typhoon_engine::core::strategy_metrics::{
    MetricResult, MetricValue, StrategyAnalysis, TradeDirection, UndefinedReason,
};
use typhoon_engine::core::strategy_report::{
    MAX_REPORT_ARTIFACT_JSON_BYTES, StrategyReportArtifact,
};
use typhoon_engine::core::strategy_simulator::{
    ClientOrderId, FillRecord, ModifyRequest, OrderKind, OrderRequest, OrderSide, SimEventKind,
    SimulationReport, SymbolId,
};

pub(crate) const MAX_PREPARED_REPORT_ITEMS: usize = 250_000;
pub(crate) const MAX_COMPARISON_RUNS: usize = 4;
const DEFAULT_DISTRIBUTION_BINS: usize = 24;
const MAX_SIMULATION_REPORT_JSON_BYTES: usize = 64 * 1024 * 1024;
const MAX_COMPARISON_TOTAL_JSON_BYTES: usize =
    MAX_COMPARISON_RUNS * (MAX_SIMULATION_REPORT_JSON_BYTES + MAX_REPORT_ARTIFACT_JSON_BYTES);

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PreparedBin {
    pub(crate) lower: f64,
    pub(crate) upper: f64,
    pub(crate) count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedDistribution {
    pub(crate) label: String,
    pub(crate) units: String,
    pub(crate) bins: Vec<PreparedBin>,
    pub(crate) observations: usize,
}

pub(crate) fn prepare_distribution(
    label: &str,
    units: &str,
    values: &[f64],
    requested_bins: usize,
) -> Result<PreparedDistribution, String> {
    if values.len() > MAX_PREPARED_REPORT_ITEMS {
        return Err(format!(
            "distribution has {} observations (limit {MAX_PREPARED_REPORT_ITEMS})",
            values.len()
        ));
    }
    if requested_bins == 0 || requested_bins > DEFAULT_DISTRIBUTION_BINS {
        return Err(format!(
            "distribution bin count must be in 1..={DEFAULT_DISTRIBUTION_BINS}"
        ));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err("distribution observations must be finite".into());
    }
    if values.is_empty() {
        return Ok(PreparedDistribution {
            label: label.into(),
            units: units.into(),
            bins: Vec::new(),
            observations: 0,
        });
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if min == max {
        return Ok(PreparedDistribution {
            label: label.into(),
            units: units.into(),
            bins: vec![PreparedBin {
                lower: min,
                upper: max,
                count: values.len(),
            }],
            observations: values.len(),
        });
    }
    let width = (max - min) / requested_bins as f64;
    if !width.is_finite() || width <= 0.0 {
        return Err("distribution range cannot be represented".into());
    }
    let mut bins: Vec<_> = (0..requested_bins)
        .map(|index| PreparedBin {
            lower: min + width * index as f64,
            upper: if index + 1 == requested_bins {
                max
            } else {
                min + width * (index + 1) as f64
            },
            count: 0,
        })
        .collect();
    for value in values {
        let index = if *value == max {
            requested_bins - 1
        } else {
            ((*value - min) / width).floor() as usize
        };
        bins[index.min(requested_bins - 1)].count += 1;
    }
    Ok(PreparedDistribution {
        label: label.into(),
        units: units.into(),
        bins,
        observations: values.len(),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PairedMetricDelta {
    Defined(f64),
    NotComparable {
        baseline: MetricValue,
        candidate: MetricValue,
    },
}

pub(crate) fn paired_metric_delta(
    baseline: &MetricValue,
    candidate: &MetricValue,
) -> PairedMetricDelta {
    match (baseline, candidate) {
        (MetricValue::Defined { value: left }, MetricValue::Defined { value: right }) => {
            PairedMetricDelta::Defined(right - left)
        }
        _ => PairedMetricDelta::NotComparable {
            baseline: baseline.clone(),
            candidate: candidate.clone(),
        },
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedMetricComparison {
    pub(crate) id: String,
    pub(crate) values: Vec<MetricValue>,
    pub(crate) deltas_from_baseline: Vec<PairedMetricDelta>,
}

#[derive(Clone, Debug)]
pub(crate) struct StrategyComparisonView {
    pub(crate) runs: Vec<StrategyResultView>,
    pub(crate) metrics: Vec<PreparedMetricComparison>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ComparisonLoadState {
    generation: u64,
    active: Option<u64>,
}

impl ComparisonLoadState {
    pub(crate) fn begin_request(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.active = Some(self.generation);
        self.generation
    }

    pub(crate) fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.active = None;
    }

    pub(crate) fn accept(&mut self, request: u64, run_count: usize) -> bool {
        if self.active != Some(request) || !(2..=MAX_COMPARISON_RUNS).contains(&run_count) {
            return false;
        }
        self.active = None;
        true
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.active.is_some()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedRunIdentity {
    pub(crate) manifest_evidence_available: bool,
    pub(crate) intervention_log_id: Option<String>,
    pub(crate) sub_bar_bindings: Vec<(String, String)>,
    pub(crate) repaint_bindings: Vec<PreparedRepaintBinding>,
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedRepaintBinding {
    pub(crate) indicator_id: String,
    pub(crate) artifact_id: String,
    pub(crate) warning_note: Option<String>,
}

pub(crate) fn run_evidence_labels(view: &StrategyResultView) -> Vec<String> {
    let mut labels = vec![
        format!("run {}", view.run_id),
        format!("report {}", view.report_id),
    ];
    if !view.identity.manifest_evidence_available {
        labels.push(
            "Manifest evidence unavailable: sealed report schema v1 preserves only canonical run/report identity."
                .into(),
        );
        return labels;
    }
    labels.push(match &view.identity.intervention_log_id {
        Some(identity) => format!("Hybrid intervention log: {identity}"),
        None => "Automated run: no intervention log bound".into(),
    });
    if view.identity.sub_bar_bindings.is_empty() {
        labels.push("Fidelity identity: no sub-bar dataset bound".into());
    } else {
        labels.extend(
            view.identity
                .sub_bar_bindings
                .iter()
                .map(|(parent, dataset)| format!("Sub-bar dataset {parent}: {dataset}")),
        );
    }
    if view.identity.repaint_bindings.is_empty() {
        labels.push("Repaint QA: no indicator artifact bound".into());
    } else {
        labels.extend(view.identity.repaint_bindings.iter().map(|binding| {
            binding.warning_note.as_ref().map_or_else(
                || {
                    format!(
                        "CLEAN · indicator {} · artifact {}",
                        binding.indicator_id, binding.artifact_id
                    )
                },
                |note| {
                    format!(
                        "WARNING ACKNOWLEDGED · indicator {} · artifact {} · {note}",
                        binding.indicator_id, binding.artifact_id
                    )
                },
            )
        }));
    }
    labels
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PreparedDecision {
    pub(crate) decision_index: u64,
    pub(crate) bar_index: usize,
    pub(crate) time_ns: i64,
    pub(crate) sequence: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum InterventionActionKind {
    #[default]
    SubmitMarket,
    SubmitLimit,
    SubmitStop,
    Cancel,
    ModifyQuantity,
    ModifyLimit,
    ModifyStop,
}

#[derive(Clone, Debug)]
pub(crate) struct InterventionAuthorState {
    pub(crate) selected_decision: Option<u64>,
    pub(crate) action_kind: InterventionActionKind,
    pub(crate) side: OrderSide,
    pub(crate) quantity: String,
    pub(crate) price: String,
    pub(crate) target_order_id: String,
    pub(crate) note: String,
    pub(crate) entries: Vec<Intervention>,
    pub(crate) sealed: Option<InterventionLog>,
    pub(crate) status: String,
}

impl Default for InterventionAuthorState {
    fn default() -> Self {
        Self {
            selected_decision: None,
            action_kind: InterventionActionKind::SubmitMarket,
            side: OrderSide::Buy,
            quantity: "1".into(),
            price: String::new(),
            target_order_id: String::new(),
            note: String::new(),
            entries: Vec::new(),
            sealed: None,
            status: String::new(),
        }
    }
}

impl InterventionAuthorState {
    pub(crate) fn select_decision(
        &mut self,
        selected: Option<u64>,
        revealed_decisions: &[PreparedDecision],
    ) {
        self.selected_decision = selected
            .filter(|index| {
                revealed_decisions
                    .binary_search_by_key(index, |decision| decision.decision_index)
                    .is_ok()
            })
            .or_else(|| {
                revealed_decisions
                    .last()
                    .map(|decision| decision.decision_index)
            });
    }

    pub(crate) fn parse_intervention(
        &self,
        symbol: SymbolId,
        revealed_decisions: &[PreparedDecision],
    ) -> Result<Intervention, String> {
        let decision_index = self
            .selected_decision
            .ok_or_else(|| "select a revealed decision first".to_string())?;
        if revealed_decisions
            .binary_search_by_key(&decision_index, |decision| decision.decision_index)
            .is_err()
        {
            return Err("decision is not a revealed causal decision for this symbol".into());
        }
        if self.note.len() > MAX_INTERVENTION_NOTE_BYTES {
            return Err(format!(
                "note exceeds {MAX_INTERVENTION_NOTE_BYTES} UTF-8 bytes"
            ));
        }
        let positive = |text: &str, field: &str| -> Result<f64, String> {
            let value = text
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("{field} must be a number"))?;
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("{field} must be finite and positive"));
            }
            Ok(value)
        };
        let target = || {
            self.target_order_id
                .trim()
                .parse::<u64>()
                .map(ClientOrderId)
                .map_err(|_| "target order id must be an unsigned integer".to_string())
        };
        let action = match self.action_kind {
            InterventionActionKind::SubmitMarket => InterventionAction::Submit {
                request: OrderRequest::market(
                    symbol,
                    self.side,
                    positive(&self.quantity, "quantity")?,
                ),
            },
            InterventionActionKind::SubmitLimit => InterventionAction::Submit {
                request: OrderRequest::limit(
                    symbol,
                    self.side,
                    positive(&self.quantity, "quantity")?,
                    positive(&self.price, "limit price")?,
                ),
            },
            InterventionActionKind::SubmitStop => InterventionAction::Submit {
                request: OrderRequest::stop(
                    symbol,
                    self.side,
                    positive(&self.quantity, "quantity")?,
                    positive(&self.price, "stop price")?,
                ),
            },
            InterventionActionKind::Cancel => InterventionAction::Cancel { target: target()? },
            InterventionActionKind::ModifyQuantity => InterventionAction::Modify {
                target: target()?,
                change: ModifyRequest::quantity(positive(&self.quantity, "quantity")?),
            },
            InterventionActionKind::ModifyLimit => InterventionAction::Modify {
                target: target()?,
                change: ModifyRequest::limit_price(positive(&self.price, "limit price")?),
            },
            InterventionActionKind::ModifyStop => InterventionAction::Modify {
                target: target()?,
                change: ModifyRequest::stop_price(positive(&self.price, "stop price")?),
            },
        };
        Ok(Intervention {
            decision_index,
            note: self.note.clone(),
            action,
        })
    }

    pub(crate) fn push(&mut self, intervention: Intervention) -> Result<(), String> {
        if self.entries.len() >= MAX_INTERVENTIONS {
            return Err(format!("intervention limit {MAX_INTERVENTIONS} reached"));
        }
        if self
            .entries
            .last()
            .is_some_and(|last| last.decision_index > intervention.decision_index)
        {
            return Err("interventions must be authored in decision order".into());
        }
        self.entries.push(intervention);
        self.sealed = None;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ResultSection {
    #[default]
    Summary,
    Metrics,
    Trades,
    Equity,
    Drawdown,
    Distributions,
    RunEvidence,
    Interventions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExportKind {
    Report,
    Simulation,
    Intervention,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum ExportStatus {
    #[default]
    Idle,
    Working {
        kind: ExportKind,
    },
    Saved {
        kind: ExportKind,
        identity: String,
        path: PathBuf,
    },
    Error(String),
}

impl ExportStatus {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Idle => "No export requested".into(),
            Self::Working { kind } => format!("Exporting {kind:?}…"),
            Self::Saved {
                kind,
                identity,
                path,
            } => format!(
                "Saved {kind:?} {} to {}",
                short_identity(identity),
                path.display()
            ),
            Self::Error(error) => format!("Export error: {error}"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResultWorkflowState {
    pub(crate) section: ResultSection,
    pub(crate) intervention: InterventionAuthorState,
    pub(crate) export_status: ExportStatus,
}

#[derive(Clone, Debug)]
pub(crate) enum ResultCommand {
    Seal(Vec<Intervention>),
    LoadIntervention,
    Export(ExportKind),
}

pub(crate) enum WorkflowWorkerResult {
    Sealed(Result<InterventionLog, String>),
    Loaded(Result<InterventionLog, String>),
    Exported {
        kind: ExportKind,
        identity: String,
        path: PathBuf,
        result: Result<(), String>,
    },
}

pub(crate) enum ExportSource {
    /// Exact source bytes retained only after report-pair identity verification.
    VerifiedBytes(Vec<u8>),
    /// Serialization re-verifies the content address on the export worker.
    Intervention(InterventionLog),
}

pub(crate) fn export_verified_source(path: &Path, source: ExportSource) -> Result<(), String> {
    let bytes = match source {
        ExportSource::VerifiedBytes(bytes) => {
            if bytes.is_empty() || bytes.len() > MAX_SIMULATION_REPORT_JSON_BYTES {
                return Err(format!(
                    "verified export has {} bytes (allowed 1..={MAX_SIMULATION_REPORT_JSON_BYTES})",
                    bytes.len()
                ));
            }
            bytes
        }
        ExportSource::Intervention(log) => log.to_json_vec().map_err(|error| error.to_string())?,
    };
    std::fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

pub(crate) fn load_intervention_log(
    path: &Path,
    decisions: &[PreparedDecision],
) -> Result<InterventionLog, String> {
    let bytes = read_bounded(
        path,
        typhoon_engine::core::strategy_intervention::MAX_INTERVENTION_LOG_JSON_BYTES,
    )?;
    let log = InterventionLog::from_json_slice(&bytes).map_err(|error| error.to_string())?;
    seal_interventions(log.interventions().to_vec(), decisions)?;
    Ok(log)
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
    pub(crate) decisions: Vec<PreparedDecision>,
    pub(crate) metrics: Vec<MetricResult>,
    pub(crate) distributions: Vec<PreparedDistribution>,
    pub(crate) identity: PreparedRunIdentity,
    pub(crate) diagnostics: typhoon_engine::core::strategy_metrics::Diagnostics,
    /// Exact already-verified source bytes retained for explicit user exports.
    pub(crate) report_artifact_json: Vec<u8>,
    pub(crate) simulation_report_json: Vec<u8>,
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
        let decisions = report
            .events
            .iter()
            .filter(|event| event.kind == SimEventKind::Decision)
            .enumerate()
            .filter(|(_, event)| event.symbol == Some(symbol_id))
            .map(|(decision_index, event)| PreparedDecision {
                decision_index: decision_index as u64,
                bar_index: bar_index_for_time_ns(chart_bar_times_ms, event.time_ns),
                time_ns: event.time_ns,
                sequence: event.sequence,
            })
            .collect();
        let trade_durations = analysis
            .trades
            .iter()
            .map(|trade| {
                trade
                    .exit_time_ns
                    .checked_sub(trade.entry_time_ns)
                    .filter(|duration| *duration >= 0)
                    .map(|duration| duration as f64 / 1_000_000_000.0)
                    .ok_or_else(|| {
                        StrategyResultViewError::Verification(
                            "closed trade has an invalid negative/overflowing duration".into(),
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let distribution_inputs = [
            (
                "Daily close-to-close return",
                "fraction of prior calendar close (first day uses initial equity)",
                analysis
                    .calendar
                    .daily
                    .iter()
                    .filter_map(|point| point.return_fraction)
                    .collect::<Vec<_>>(),
            ),
            (
                "Closed-trade holding duration",
                "seconds from opening fill to closing fill",
                trade_durations,
            ),
            (
                "Maximum adverse excursion",
                "non-negative entry-relative account currency; fully contained closed bars",
                analysis.trades.iter().map(|trade| trade.mae).collect(),
            ),
            (
                "Maximum favorable excursion",
                "non-negative entry-relative account currency; fully contained closed bars",
                analysis.trades.iter().map(|trade| trade.mfe).collect(),
            ),
        ];
        let distributions = distribution_inputs
            .into_iter()
            .map(|(label, units, values)| {
                prepare_distribution(label, units, &values, DEFAULT_DISTRIBUTION_BINS)
                    .map_err(StrategyResultViewError::Verification)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let identity = artifact.run_manifest().map_or_else(
            || PreparedRunIdentity {
                manifest_evidence_available: false,
                intervention_log_id: None,
                sub_bar_bindings: Vec::new(),
                repaint_bindings: Vec::new(),
            },
            |manifest| {
                let binding = manifest.binding();
                PreparedRunIdentity {
                    manifest_evidence_available: true,
                    intervention_log_id: binding.intervention_log_id.clone(),
                    sub_bar_bindings: binding
                        .sub_bar_datasets
                        .iter()
                        .map(|item| (item.parent_input_id.clone(), item.dataset_id.clone()))
                        .collect(),
                    repaint_bindings: binding
                        .repaint_qa
                        .iter()
                        .map(|item| PreparedRepaintBinding {
                            indicator_id: item.indicator_id.clone(),
                            artifact_id: item.artifact_id.clone(),
                            warning_note: match &item.acknowledgement {
                                RepaintAcknowledgement::Clean => None,
                                RepaintAcknowledgement::WarningAcknowledged { note } => {
                                    Some(note.clone())
                                }
                            },
                        })
                        .collect(),
                }
            },
        );

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
            decisions,
            metrics: analysis.metrics.clone(),
            distributions,
            identity,
            diagnostics: analysis.diagnostics.clone(),
            report_artifact_json: Vec::new(),
            simulation_report_json: Vec::new(),
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
    let (artifact, artifact_bytes, report_bytes) = match (first_artifact, second_artifact) {
        (Some(artifact), None) => (artifact, first.as_slice(), second.as_slice()),
        (None, Some(artifact)) => (artifact, second.as_slice(), first.as_slice()),
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
    let mut view = StrategyResultView::prepare(&artifact, &report, symbol_id, chart_bar_times_ms)
        .map_err(|error| error.to_string())?;
    view.report_artifact_json = artifact_bytes.to_vec();
    view.simulation_report_json = report_bytes.to_vec();
    Ok(view)
}

/// Load two to four verified report/simulation pairs in graphical selection order.
/// Parsing, digest matching, distribution binning and delta preparation belong on
/// the caller's bounded worker; rendering only indexes the returned vectors.
pub(crate) fn load_prepared_comparison(
    paths: Vec<PathBuf>,
    chart_symbol: &str,
    chart_bar_times_ms: &[i64],
) -> Result<StrategyComparisonView, String> {
    if paths.len() < 4 || paths.len() > MAX_COMPARISON_RUNS * 2 || paths.len() % 2 != 0 {
        return Err(format!(
            "select 2..={MAX_COMPARISON_RUNS} complete report/simulation pairs"
        ));
    }
    let mut artifacts = Vec::new();
    let mut reports = Vec::new();
    let mut total_bytes = 0_usize;
    for (selection_index, path) in paths.into_iter().enumerate() {
        let bytes = read_bounded(&path, MAX_SIMULATION_REPORT_JSON_BYTES)?;
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| "comparison byte count overflowed".to_string())?;
        if total_bytes > MAX_COMPARISON_TOTAL_JSON_BYTES {
            return Err(format!(
                "comparison inputs exceed total byte limit {MAX_COMPARISON_TOTAL_JSON_BYTES}"
            ));
        }
        if let Ok(artifact) = StrategyReportArtifact::from_json_slice(&bytes) {
            artifacts.push((selection_index, artifact, bytes));
        } else {
            let report: SimulationReport = serde_json::from_slice(&bytes).map_err(|error| {
                format!(
                    "{} is neither a verified report artifact nor a SimulationReport: {error}",
                    path.display()
                )
            })?;
            reports.push(Some((report, bytes)));
        }
    }
    if artifacts.len() != reports.len() || !(2..=MAX_COMPARISON_RUNS).contains(&artifacts.len()) {
        return Err(
            "selection must contain one report artifact and one SimulationReport per run".into(),
        );
    }
    artifacts.sort_by_key(|(selection_index, _, _)| *selection_index);
    let mut run_ids = std::collections::BTreeSet::new();
    let mut report_ids = std::collections::BTreeSet::new();
    for (_, artifact, _) in &artifacts {
        if !run_ids.insert(artifact.run_id()) {
            return Err(format!(
                "duplicate run identity {}",
                short_identity(artifact.run_id())
            ));
        }
        if !report_ids.insert(artifact.report_id()) {
            return Err(format!(
                "duplicate report identity {}",
                short_identity(artifact.report_id())
            ));
        }
    }
    let mut runs = Vec::with_capacity(artifacts.len());
    for (_, artifact, artifact_bytes) in artifacts {
        let matches: Vec<_> = reports
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                candidate
                    .as_ref()
                    .filter(|(report, _)| artifact.verify_simulation_report(report).is_ok())
                    .map(|_| index)
            })
            .collect();
        if matches.len() != 1 {
            return Err(format!(
                "report {} matched {} selected simulations; expected exactly one",
                short_identity(artifact.report_id()),
                matches.len()
            ));
        }
        let (report, report_bytes) = reports[matches[0]]
            .take()
            .ok_or_else(|| "simulation pair was selected twice".to_string())?;
        let symbol_id = report
            .symbols
            .iter()
            .position(|symbol| symbols_match(symbol, chart_symbol))
            .map(SymbolId)
            .ok_or_else(|| format!("report has no symbol matching active chart {chart_symbol}"))?;
        let mut view =
            StrategyResultView::prepare(&artifact, &report, symbol_id, chart_bar_times_ms)
                .map_err(|error| error.to_string())?;
        view.report_artifact_json = artifact_bytes;
        view.simulation_report_json = report_bytes;
        runs.push(view);
    }
    let baseline = &runs[0];
    let baseline_metric_ids: Vec<_> = baseline.metrics.iter().map(|metric| &metric.id).collect();
    if runs.iter().skip(1).any(|run| {
        run.metrics
            .iter()
            .map(|metric| &metric.id)
            .collect::<Vec<_>>()
            != baseline_metric_ids
    }) {
        return Err("comparison registry metric order is inconsistent between reports".into());
    }
    let metrics = baseline
        .metrics
        .iter()
        .map(|metric| {
            let values: Vec<_> = runs
                .iter()
                .map(|run| {
                    run.metric(&metric.id).cloned().ok_or_else(|| {
                        format!(
                            "run {} lacks registry metric {}",
                            short_identity(&run.run_id),
                            metric.id
                        )
                    })
                })
                .collect::<Result<_, _>>()?;
            let deltas_from_baseline = values
                .iter()
                .skip(1)
                .map(|value| paired_metric_delta(&values[0], value))
                .collect();
            Ok(PreparedMetricComparison {
                id: metric.id.clone(),
                values,
                deltas_from_baseline,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(StrategyComparisonView { runs, metrics })
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let file_len = file
        .metadata()
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
        .len();
    if file_len > limit as u64 {
        return Err(format!(
            "{} exceeds byte limit {limit} ({file_len} bytes)",
            path.display()
        ));
    }
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() > limit {
        return Err(format!("{} exceeds byte limit {limit}", path.display()));
    }
    Ok(bytes)
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

pub(crate) fn revealed_decision_count(decisions: &[PreparedDecision], replay_bar: usize) -> usize {
    decisions.partition_point(|decision| decision.bar_index < replay_bar)
}

pub(crate) fn decision_replay_bar(
    decisions: &[PreparedDecision],
    decision_index: u64,
) -> Option<usize> {
    decisions
        .binary_search_by_key(&decision_index, |decision| decision.decision_index)
        .ok()
        .and_then(|position| decisions.get(position))
        .map(|decision| decision.bar_index.saturating_add(1))
}

pub(crate) fn seal_interventions(
    entries: Vec<Intervention>,
    decisions: &[PreparedDecision],
) -> Result<InterventionLog, String> {
    if let Some(entry) = entries.iter().find(|entry| {
        decisions
            .binary_search_by_key(&entry.decision_index, |decision| decision.decision_index)
            .is_err()
    }) {
        return Err(format!(
            "intervention decision {} is not a causal decision for the selected symbol",
            entry.decision_index
        ));
    }
    InterventionLog::build(entries).map_err(|error| error.to_string())
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

fn render_curve(ui: &mut egui::Ui, id: &str, points: &[PreparedCurvePoint]) {
    if points.len() < 2 {
        ui.label("Not enough points.");
        return;
    }
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(120.0), 150.0),
        egui::Sense::hover(),
    );
    let min = points
        .iter()
        .map(|point| point.value)
        .fold(f64::INFINITY, f64::min);
    let max = points
        .iter()
        .map(|point| point.value)
        .fold(f64::NEG_INFINITY, f64::max);
    let span = (max - min).max(f64::EPSILON);
    let denominator = (points.len() - 1) as f32;
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(18, 20, 28));
    for (index, pair) in points.windows(2).enumerate() {
        let map = |offset: usize, value: f64| {
            egui::pos2(
                rect.left() + (index + offset) as f32 / denominator * rect.width(),
                rect.bottom() - ((value - min) / span) as f32 * rect.height(),
            )
        };
        painter.line_segment(
            [map(0, pair[0].value), map(1, pair[1].value)],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(52, 152, 219)),
        );
    }
    ui.label(format!(
        "{id}: {min:.2} … {max:.2} · {} points",
        points.len()
    ));
}

fn render_distribution(ui: &mut egui::Ui, distribution: &PreparedDistribution) {
    ui.strong(format!(
        "{} · {} observations · {}",
        distribution.label, distribution.observations, distribution.units
    ));
    if distribution.bins.is_empty() {
        ui.label("No defined observations.");
        return;
    }
    let max_count = distribution
        .bins
        .iter()
        .map(|bin| bin.count)
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(120.0), 110.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(18, 20, 28));
    let width = rect.width() / distribution.bins.len() as f32;
    for (index, bin) in distribution.bins.iter().enumerate() {
        let height = rect.height() * bin.count as f32 / max_count;
        let bar = egui::Rect::from_min_max(
            egui::pos2(
                rect.left() + index as f32 * width + 1.0,
                rect.bottom() - height,
            ),
            egui::pos2(
                rect.left() + (index + 1) as f32 * width - 1.0,
                rect.bottom(),
            ),
        );
        painter.rect_filled(bar, 1.0, egui::Color32::from_rgb(52, 152, 219));
    }
    if let (Some(first), Some(last)) = (distribution.bins.first(), distribution.bins.last()) {
        ui.small(format!("{:.4} … {:.4}", first.lower, last.upper));
    }
}

fn render_result_section(
    ui: &mut egui::Ui,
    view: &StrategyResultView,
    workflow: &mut ResultWorkflowState,
    replay_active: &mut bool,
    replay_bar_idx: &mut usize,
    replay_playing: &mut bool,
    commands: &mut Vec<ResultCommand>,
) {
    if *replay_active && completed_section_hidden_in_replay(workflow.section) {
        ui.label(
            egui::RichText::new(
                "Completed-run trades, curves, and metrics are hidden until replay exits.",
            )
            .color(egui::Color32::from_rgb(255, 200, 50)),
        );
        return;
    }
    match workflow.section {
        ResultSection::Summary => {
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
        }
        ResultSection::Metrics => {
            egui::ScrollArea::vertical().max_height(260.0).show_rows(
                ui,
                20.0,
                view.metrics.len(),
                |ui, range| {
                    for metric in &view.metrics[range] {
                        ui.horizontal(|ui| {
                            ui.label(metric.id.replace('_', " "));
                            ui.strong(format_metric_value(&metric.id, &metric.value));
                        });
                    }
                },
            );
        }
        ResultSection::Trades => {
            egui::ScrollArea::vertical().max_height(260.0).show_rows(
                ui,
                20.0,
                view.trades.len(),
                |ui, range| {
                    for trade in &view.trades[range] {
                        ui.horizontal(|ui| {
                            ui.label(format!("#{} {:?}", trade.trade_id, trade.direction));
                            ui.label(format!(
                                "{} → {} · qty {:.4} · P&L {:.2}",
                                trade.entry_time_ns,
                                trade.exit_time_ns,
                                trade.quantity,
                                trade.net_pnl
                            ));
                        });
                    }
                },
            );
        }
        ResultSection::Equity => render_curve(ui, "Equity", &view.equity),
        ResultSection::Drawdown => render_curve(ui, "Drawdown", &view.drawdown),
        ResultSection::Distributions => {
            egui::ScrollArea::vertical()
                .max_height(420.0)
                .show(ui, |ui| {
                    for distribution in &view.distributions {
                        render_distribution(ui, distribution);
                        ui.separator();
                    }
                });
        }
        ResultSection::RunEvidence => {
            for label in run_evidence_labels(view) {
                if label.starts_with("WARNING ACKNOWLEDGED") {
                    ui.label(
                        egui::RichText::new(label).color(egui::Color32::from_rgb(255, 200, 50)),
                    );
                } else {
                    ui.label(label);
                }
            }
        }
        ResultSection::Interventions => {
            let author = &mut workflow.intervention;
            if !*replay_active || *replay_playing {
                ui.label(
                    egui::RichText::new("Pause verified replay to author at a revealed decision.")
                        .color(egui::Color32::from_rgb(255, 200, 50)),
                );
                return;
            }
            let revealed = revealed_decision_count(&view.decisions, *replay_bar_idx);
            if revealed == 0 {
                ui.label("No decision is revealed at this replay position.");
                return;
            }
            let revealed_decisions = &view.decisions[..revealed];
            let latest = revealed_decisions
                .last()
                .map(|decision| decision.decision_index);
            author.select_decision(author.selected_decision.or(latest), revealed_decisions);
            ui.horizontal(|ui| {
                let selected_position = author.selected_decision.and_then(|selected| {
                    revealed_decisions
                        .binary_search_by_key(&selected, |decision| decision.decision_index)
                        .ok()
                });
                if ui.button("Previous decision").clicked() {
                    let position = selected_position.unwrap_or(revealed - 1).saturating_sub(1);
                    author.selected_decision = Some(revealed_decisions[position].decision_index);
                }
                if ui.button("Next decision").clicked() {
                    let position = selected_position
                        .unwrap_or(0)
                        .saturating_add(1)
                        .min(revealed - 1);
                    author.selected_decision = Some(revealed_decisions[position].decision_index);
                }
                if let Some(index) = author.selected_decision {
                    if let Some(bar) = decision_replay_bar(&view.decisions, index) {
                        *replay_bar_idx = bar;
                    }
                    let manual = author
                        .entries
                        .iter()
                        .filter(|entry| entry.decision_index == index)
                        .count();
                    let badge = if manual == 0 {
                        "AUTOMATED".to_string()
                    } else {
                        format!("MANUAL ×{manual}")
                    };
                    let color = if manual == 0 {
                        egui::Color32::from_rgb(52, 152, 219)
                    } else {
                        egui::Color32::from_rgb(241, 196, 15)
                    };
                    ui.label(
                        egui::RichText::new(format!("decision {index} · {badge}")).color(color),
                    );
                }
            });
            egui::ComboBox::from_label("Action")
                .selected_text(format!("{:?}", author.action_kind))
                .show_ui(ui, |ui| {
                    for kind in [
                        InterventionActionKind::SubmitMarket,
                        InterventionActionKind::SubmitLimit,
                        InterventionActionKind::SubmitStop,
                        InterventionActionKind::Cancel,
                        InterventionActionKind::ModifyQuantity,
                        InterventionActionKind::ModifyLimit,
                        InterventionActionKind::ModifyStop,
                    ] {
                        ui.selectable_value(&mut author.action_kind, kind, format!("{kind:?}"));
                    }
                });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut author.side, OrderSide::Buy, "Buy");
                ui.selectable_value(&mut author.side, OrderSide::Sell, "Sell");
                ui.label("Qty");
                ui.add(egui::TextEdit::singleline(&mut author.quantity).desired_width(70.0));
                ui.label("Price");
                ui.add(egui::TextEdit::singleline(&mut author.price).desired_width(90.0));
                ui.label("Target order");
                ui.add(egui::TextEdit::singleline(&mut author.target_order_id).desired_width(90.0));
            });
            ui.add(
                egui::TextEdit::multiline(&mut author.note)
                    .hint_text("Bounded operator note / rationale")
                    .desired_rows(2),
            );
            ui.label(format!(
                "{} / {MAX_INTERVENTION_NOTE_BYTES} UTF-8 bytes",
                author.note.len()
            ));
            ui.horizontal(|ui| {
                if ui.button("Add manual action").clicked() {
                    let result = author
                        .parse_intervention(view.symbol_id, &view.decisions[..revealed])
                        .and_then(|entry| author.push(entry));
                    author.status = match result {
                        Ok(()) => "Manual action added; seal is now stale.".into(),
                        Err(error) => format!("Error: {error}"),
                    };
                }
                if ui
                    .add_enabled(
                        !author.entries.is_empty(),
                        egui::Button::new("Seal intervention log"),
                    )
                    .clicked()
                {
                    commands.push(ResultCommand::Seal(author.entries.clone()));
                    author.status = "Sealing and validating off the render thread…".into();
                }
            });
            if let Some(log) = &author.sealed {
                ui.label(
                    egui::RichText::new(format!(
                        "SEALED DRAFT {} · {} actions · not replay-verified; bind this log id in a new run manifest and rerun",
                        short_identity(log.log_id()),
                        log.interventions().len()
                    ))
                    .color(egui::Color32::from_rgb(46, 204, 113)),
                );
            }
            ui.label(&author.status);
            egui::ScrollArea::vertical().max_height(160.0).show_rows(
                ui,
                20.0,
                author.entries.len(),
                |ui, range| {
                    for entry in &author.entries[range] {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("MANUAL")
                                    .color(egui::Color32::from_rgb(241, 196, 15)),
                            );
                            ui.label(format!(
                                "decision {} · {:?} · {}",
                                entry.decision_index, entry.action, entry.note
                            ));
                        });
                    }
                },
            );
        }
    }
}

const fn completed_section_hidden_in_replay(section: ResultSection) -> bool {
    matches!(
        section,
        ResultSection::Summary
            | ResultSection::Metrics
            | ResultSection::Trades
            | ResultSection::Equity
            | ResultSection::Drawdown
            | ResultSection::Distributions
            | ResultSection::RunEvidence
    )
}

pub(crate) fn render_prepared_result(
    ui: &mut egui::Ui,
    view: &StrategyResultView,
    workflow: &mut ResultWorkflowState,
    selected: &mut Option<usize>,
    replay_active: &mut bool,
    replay_bar_idx: &mut usize,
    replay_playing: &mut bool,
    replay_speed: &mut f32,
) -> Vec<ResultCommand> {
    let mut commands = Vec::new();
    ui.separator();
    ui.heading("Verified Strategy Report");
    ui.horizontal_wrapped(|ui| {
        ui.strong(&view.symbol);
        ui.label(format!("symbol #{}", view.symbol_id.0));
        ui.label(format!("run {}", short_identity(&view.run_id)));
        ui.label(format!("report {}", short_identity(&view.report_id)));
        ui.label(format!("metrics {}", view.metrics_version));
    });
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                !*replay_active,
                egui::Button::new("Export report artifact…"),
            )
            .clicked()
        {
            commands.push(ResultCommand::Export(ExportKind::Report));
        }
        if ui
            .add_enabled(!*replay_active, egui::Button::new("Export simulation…"))
            .clicked()
        {
            commands.push(ResultCommand::Export(ExportKind::Simulation));
        }
        if ui.button("Load intervention log…").clicked() {
            commands.push(ResultCommand::LoadIntervention);
        }
        if ui
            .add_enabled(
                workflow.intervention.sealed.is_some(),
                egui::Button::new("Export sealed intervention…"),
            )
            .clicked()
        {
            commands.push(ResultCommand::Export(ExportKind::Intervention));
        }
    });
    ui.label(workflow.export_status.label());
    ui.horizontal_wrapped(|ui| {
        for (section, label) in [
            (ResultSection::Summary, "Summary"),
            (ResultSection::Metrics, "Metrics"),
            (ResultSection::Trades, "Trades"),
            (ResultSection::Equity, "Equity"),
            (ResultSection::Drawdown, "Drawdown"),
            (ResultSection::Distributions, "Distributions"),
            (ResultSection::RunEvidence, "Run evidence"),
            (ResultSection::Interventions, "Interventions"),
        ] {
            ui.selectable_value(&mut workflow.section, section, label);
        }
    });

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

    render_result_section(
        ui,
        view,
        workflow,
        replay_active,
        replay_bar_idx,
        replay_playing,
        &mut commands,
    );

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
        return commands;
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
            ui.label("Open · future exit, P&L, MAE, and MFE are hidden.");
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
    commands
}

pub(crate) fn render_comparison(
    ui: &mut egui::Ui,
    comparison: &StrategyComparisonView,
    selected_run: &mut usize,
) -> bool {
    *selected_run = (*selected_run).min(comparison.runs.len().saturating_sub(1));
    ui.separator();
    ui.heading("Verified report comparison");
    ui.small(
        "Selection-order baseline; deltas exist only when both typed metric values are defined.",
    );
    ui.horizontal_wrapped(|ui| {
        for (index, run) in comparison.runs.iter().enumerate() {
            let label = format!(
                "{} {} / {}",
                if index == 0 { "Baseline" } else { "Run" },
                short_identity(&run.run_id),
                short_identity(&run.report_id)
            );
            ui.selectable_value(selected_run, index, label);
        }
    });
    let inspect = ui
        .button("Inspect selected on existing chart overlay")
        .clicked();
    egui::ScrollArea::vertical()
        .id_salt("strategy_comparison_metrics")
        .max_height(300.0)
        .show_rows(ui, 20.0, comparison.metrics.len(), |ui, range| {
            for metric in &comparison.metrics[range] {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(metric.id.replace('_', " "));
                    ui.label(format!(
                        "baseline {}",
                        format_metric_value(&metric.id, &metric.values[0])
                    ));
                    for (candidate_index, delta) in metric.deltas_from_baseline.iter().enumerate() {
                        match delta {
                            PairedMetricDelta::Defined(value) => {
                                let prefix = if *value > 0.0 { "+" } else { "" };
                                ui.label(format!(
                                    "run {} Δ {prefix}{}",
                                    candidate_index + 2,
                                    format_metric_value(&metric.id, &MetricValue::defined(*value))
                                ));
                            }
                            PairedMetricDelta::NotComparable {
                                baseline,
                                candidate,
                            } => {
                                ui.label(format!(
                                    "run {} Δ not comparable ({} vs {})",
                                    candidate_index + 2,
                                    format_metric_value(&metric.id, baseline),
                                    format_metric_value(&metric.id, candidate)
                                ));
                            }
                        }
                    }
                });
            }
        });
    if let Some(run) = comparison.runs.get(*selected_run) {
        ui.collapsing("Selected run distributions", |ui| {
            for distribution in &run.distributions {
                render_distribution(ui, distribution);
            }
        });
    }
    inspect
}

pub(crate) fn comparison_run_to_install(
    comparison: &StrategyComparisonView,
    selected_run: usize,
) -> Option<StrategyResultView> {
    comparison.runs.get(selected_run).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use typhoon_engine::core::strategy_ir::{DatasetBinding, RunBinding, StrategyRunManifest};
    use typhoon_engine::core::strategy_metrics::{
        METRICS_SCHEMA_VERSION, MetricValue, TradeDirection, UndefinedReason, metric_registry,
    };
    use typhoon_engine::core::strategy_simulator::{
        ClientOrderId, EquityPoint, FillRecord, OrderSide, SymbolId,
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

    struct ComparisonFiles {
        paths: Vec<PathBuf>,
        dir: PathBuf,
    }

    impl Drop for ComparisonFiles {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn comparison_manifest(seed: u64) -> StrategyRunManifest {
        StrategyRunManifest::build(&RunBinding {
            datasets: vec![DatasetBinding {
                input_id: "primary".into(),
                dataset_id: format!("{seed:064x}"),
            }],
            sub_bar_datasets: vec![],
            strategy_id: "b".repeat(64),
            config_id: "c".repeat(64),
            seed,
            engine_version: "typhoon-native/comparison-test".into(),
            metrics_version: METRICS_SCHEMA_VERSION.into(),
            intervention_log_id: None,
            repaint_qa: vec![],
        })
        .expect("manifest")
    }

    fn comparison_report(seed: u64) -> SimulationReport {
        let final_equity = 1_000.0 + seed as f64;
        SimulationReport {
            symbols: vec!["TEST".into()],
            events: vec![],
            fills: vec![],
            rejections: vec![],
            cancellations: vec![],
            pending_orders: vec![],
            positions: vec![],
            equity_curve: vec![
                EquityPoint {
                    time_ns: 0,
                    sequence: 0,
                    cash: 1_000.0,
                    equity: 1_000.0,
                },
                EquityPoint {
                    time_ns: 1_000_000_000,
                    sequence: 1,
                    cash: final_equity,
                    equity: final_equity,
                },
            ],
            financing_charges: vec![],
            corporate_actions: vec![],
            final_cash: final_equity,
            final_equity,
            final_realized_pnl: 0.0,
            total_commission: 0.0,
            total_conversion_cost: 0.0,
            total_financing_cost: 0.0,
        }
    }

    fn comparison_files(run_count: usize, tag: &str) -> ComparisonFiles {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "typhoon-comparison-{tag}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("fixture dir");
        let mut paths = Vec::with_capacity(run_count * 2);
        for index in 0..run_count {
            let seed = index as u64 + 1;
            let report = comparison_report(seed);
            let artifact =
                StrategyReportArtifact::build(&comparison_manifest(seed), &report, &[], 1_000.0)
                    .expect("artifact");
            let artifact_path = dir.join(format!("{index}-artifact.json"));
            let report_path = dir.join(format!("{index}-simulation.json"));
            std::fs::write(
                &artifact_path,
                artifact.to_json_vec().expect("artifact bytes"),
            )
            .expect("artifact fixture");
            std::fs::write(
                &report_path,
                serde_json::to_vec(&report).expect("report bytes"),
            )
            .expect("report fixture");
            paths.extend([artifact_path, report_path]);
        }
        ComparisonFiles { paths, dir }
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

    #[test]
    fn intervention_authoring_state_is_bounded_and_invalidates_its_seal() {
        let decisions = vec![
            PreparedDecision {
                decision_index: 2,
                bar_index: 1,
                time_ns: 2_000_000_000,
                sequence: 10,
            },
            PreparedDecision {
                decision_index: 4,
                bar_index: 3,
                time_ns: 4_000_000_000,
                sequence: 20,
            },
            PreparedDecision {
                decision_index: 7,
                bar_index: 6,
                time_ns: 7_000_000_000,
                sequence: 30,
            },
        ];
        let mut state = InterventionAuthorState::default();
        state.select_decision(Some(4), &decisions[..2]);
        state.note = "manual close".into();
        state.quantity = "1.5".into();
        state.action_kind = InterventionActionKind::SubmitMarket;
        let intervention = state
            .parse_intervention(SymbolId(0), &decisions[..2])
            .expect("valid bounded action");
        assert_eq!(intervention.decision_index, 4);
        state
            .push(intervention)
            .expect("revealed action is accepted");
        state.sealed =
            Some(seal_interventions(state.entries.clone(), &decisions).expect("candidate seals"));
        state.selected_decision = Some(7);
        assert!(
            state
                .parse_intervention(SymbolId(0), &decisions[..2])
                .is_err()
        );
        state.note = "x"
            .repeat(typhoon_engine::core::strategy_intervention::MAX_INTERVENTION_NOTE_BYTES + 1);
        state.select_decision(Some(4), &decisions[..2]);
        assert!(
            state
                .parse_intervention(SymbolId(0), &decisions[..2])
                .is_err()
        );
        state.select_decision(Some(99), &decisions);
        assert_eq!(state.selected_decision, Some(7));
    }

    #[test]
    fn verified_intervention_export_and_load_preserve_identity_and_bounds() {
        let decisions = [PreparedDecision {
            decision_index: 3,
            bar_index: 1,
            time_ns: 2_000_000_000,
            sequence: 10,
        }];
        let log = InterventionLog::build(vec![Intervention {
            decision_index: 3,
            note: "operator entry".into(),
            action: InterventionAction::Submit {
                request: OrderRequest::market(SymbolId(0), OrderSide::Buy, 1.0),
            },
        }])
        .expect("candidate seals");
        let path = std::env::temp_dir().join(format!(
            "typhoon-intervention-export-{}.json",
            std::process::id()
        ));
        export_verified_source(&path, ExportSource::Intervention(log.clone()))
            .expect("verified bytes export");
        let loaded = load_intervention_log(&path, &decisions).expect("export reloads and verifies");
        assert_eq!(loaded.log_id(), log.log_id());
        assert!(export_verified_source(&path, ExportSource::VerifiedBytes(Vec::new())).is_err());
        std::fs::remove_file(path).expect("temporary export is removed");
    }

    #[test]
    fn replay_decision_navigation_never_uses_future_events() {
        let decisions = vec![
            PreparedDecision {
                decision_index: 2,
                bar_index: 1,
                time_ns: 2_000_000_000,
                sequence: 10,
            },
            PreparedDecision {
                decision_index: 7,
                bar_index: 4,
                time_ns: 5_000_000_000,
                sequence: 20,
            },
        ];
        assert_eq!(revealed_decision_count(&decisions, 1), 0);
        assert_eq!(revealed_decision_count(&decisions, 2), 1);
        assert_eq!(revealed_decision_count(&decisions, 4), 1);
        assert_eq!(revealed_decision_count(&decisions, 5), 2);
        assert_eq!(decision_replay_bar(&decisions, 7), Some(5));
        assert_eq!(decision_replay_bar(&decisions, 1), None);
        for section in [
            ResultSection::Summary,
            ResultSection::Metrics,
            ResultSection::Trades,
            ResultSection::Equity,
            ResultSection::Drawdown,
        ] {
            assert!(completed_section_hidden_in_replay(section));
        }
        assert!(!completed_section_hidden_in_replay(
            ResultSection::Interventions
        ));
    }

    #[test]
    fn result_section_state_and_export_identity_are_explicit() {
        let mut state = ResultWorkflowState::default();
        state.section = ResultSection::Trades;
        state.export_status = ExportStatus::Saved {
            kind: ExportKind::Intervention,
            identity: "abc123".into(),
            path: PathBuf::from("log.json"),
        };
        assert_eq!(state.section, ResultSection::Trades);
        assert!(state.export_status.label().contains("abc123"));
        assert!(state.export_status.label().contains("log.json"));
    }

    #[test]
    fn hand_computed_distribution_bins_are_bounded_complete_and_ordered() {
        let distribution = prepare_distribution("return", "ratio", &[-2.0, -1.0, 0.0, 1.0, 2.0], 4)
            .expect("finite bounded distribution");
        assert_eq!(distribution.bins.len(), 4);
        assert_eq!(
            distribution.bins.iter().map(|bin| bin.count).sum::<usize>(),
            5
        );
        assert_eq!(
            distribution
                .bins
                .iter()
                .map(|bin| bin.count)
                .collect::<Vec<_>>(),
            vec![1, 1, 1, 2]
        );
        assert_eq!(distribution.bins[0].lower, -2.0);
        assert_eq!(distribution.bins[3].upper, 2.0);
        assert!(
            distribution
                .bins
                .windows(2)
                .all(|pair| pair[0].upper == pair[1].lower)
        );
        assert!(prepare_distribution("x", "ratio", &[0.0], 0).is_err());
        assert!(prepare_distribution("x", "ratio", &[f64::NAN], 4).is_err());
        assert!(
            prepare_distribution("x", "ratio", &vec![0.0; MAX_PREPARED_REPORT_ITEMS + 1], 4)
                .is_err()
        );
        assert_eq!(
            distribution,
            prepare_distribution("return", "ratio", &[-2.0, -1.0, 0.0, 1.0, 2.0], 4)
                .expect("same inputs produce identical bins")
        );
    }

    #[test]
    fn paired_metric_delta_never_assigns_meaning_to_typed_undefined_values() {
        assert_eq!(
            paired_metric_delta(&MetricValue::defined(10.0), &MetricValue::defined(12.5)),
            PairedMetricDelta::Defined(2.5)
        );
        let undefined = MetricValue::undefined(UndefinedReason::NoLosingTrades);
        assert_eq!(
            paired_metric_delta(&undefined, &MetricValue::defined(12.5)),
            PairedMetricDelta::NotComparable {
                baseline: undefined,
                candidate: MetricValue::defined(12.5),
            }
        );
    }

    #[test]
    fn comparison_request_state_rejects_stale_cancelled_and_out_of_order_results() {
        let mut state = ComparisonLoadState::default();
        let first = state.begin_request();
        let second = state.begin_request();
        assert!(!state.accept(first, 2));
        assert!(state.accept(second, 2));
        let cancelled = state.begin_request();
        state.cancel();
        assert!(!state.accept(cancelled, 2));
        let current = state.begin_request();
        assert!(!state.accept(current, MAX_COMPARISON_RUNS + 1));
    }

    #[test]
    fn two_verified_pairs_load_in_artifact_selection_order_with_all_metrics_and_evidence() {
        let files = comparison_files(2, "two");
        let comparison = load_prepared_comparison(files.paths.clone(), "TEST", &[0, 1_000])
            .expect("two complete pairs load");
        assert_eq!(comparison.runs.len(), 2);
        assert_eq!(comparison.metrics.len(), metric_registry().len());
        assert_eq!(comparison.metrics[0].values.len(), 2);
        assert_eq!(comparison.metrics[0].deltas_from_baseline.len(), 1);
        assert!(
            comparison
                .runs
                .iter()
                .all(|run| run.identity.manifest_evidence_available)
        );
        let labels = run_evidence_labels(&comparison.runs[0]);
        assert!(
            labels
                .iter()
                .any(|label| label == &format!("run {}", comparison.runs[0].run_id))
        );
        assert!(
            labels
                .iter()
                .any(|label| label == &format!("report {}", comparison.runs[0].report_id))
        );
        assert!(
            labels
                .iter()
                .any(|label| label == "Automated run: no intervention log bound")
        );
        assert_eq!(
            comparison.runs[0].distributions[0].label,
            "Daily close-to-close return"
        );
        assert!(
            comparison.runs[0].distributions[0]
                .units
                .contains("initial equity")
        );
    }

    #[test]
    fn four_verified_pairs_load_and_selected_run_installs_without_reordering() {
        let files = comparison_files(4, "four");
        let comparison = load_prepared_comparison(files.paths.clone(), "TEST", &[0, 1_000])
            .expect("four complete pairs load");
        assert_eq!(comparison.runs.len(), MAX_COMPARISON_RUNS);
        assert!(comparison.metrics.iter().all(|metric| {
            metric.values.len() == MAX_COMPARISON_RUNS
                && metric.deltas_from_baseline.len() == MAX_COMPARISON_RUNS - 1
        }));
        let installed = comparison_run_to_install(&comparison, 3).expect("fourth run selected");
        assert_eq!(installed.run_id, comparison.runs[3].run_id);
        assert_eq!(installed.report_id, comparison.runs[3].report_id);
        assert!(comparison_run_to_install(&comparison, 4).is_none());
    }

    #[test]
    fn comparison_rejects_malformed_counts_duplicate_identities_and_ambiguous_pairing() {
        let missing = PathBuf::from("not-read-because-count-is-invalid");
        assert!(
            load_prepared_comparison(vec![missing.clone(); 3], "TEST", &[0, 1_000])
                .expect_err("odd count rejected before I/O")
                .contains("complete report/simulation pairs")
        );

        let duplicate = comparison_files(2, "duplicate");
        std::fs::copy(&duplicate.paths[0], &duplicate.paths[2]).expect("duplicate artifact");
        let error = load_prepared_comparison(duplicate.paths.clone(), "TEST", &[0, 1_000])
            .expect_err("duplicate identity rejected");
        assert!(error.contains("duplicate run identity"));

        let ambiguous = comparison_files(2, "ambiguous");
        std::fs::copy(&ambiguous.paths[1], &ambiguous.paths[3]).expect("duplicate simulation");
        let error = load_prepared_comparison(ambiguous.paths.clone(), "TEST", &[0, 1_000])
            .expect_err("one artifact cannot match two selected simulations");
        assert!(error.contains("matched 2 selected simulations"));
    }

    #[test]
    fn comparison_file_reads_are_hard_byte_bounded_before_decode() {
        let path = std::env::temp_dir().join(format!(
            "typhoon-comparison-oversize-{}",
            std::process::id()
        ));
        let file = std::fs::File::create(&path).expect("sparse fixture");
        file.set_len(MAX_SIMULATION_REPORT_JSON_BYTES as u64 + 1)
            .expect("sparse oversized file");
        let error = read_bounded(&path, MAX_SIMULATION_REPORT_JSON_BYTES)
            .expect_err("metadata limit rejects before allocation/decode");
        let _ = std::fs::remove_file(path);
        assert!(error.contains("exceeds byte limit"));
    }
}
