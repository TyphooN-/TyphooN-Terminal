//! Bounded read-only presentation of a sealed ADR-135 §7.6 problem-recognition verdict.
//!
//! Decoding one of these artifacts is a complete replay: `from_json_slice` zstd-decompresses the
//! three sealed source studies it embeds and re-derives every observation, gate and verdict before
//! it will hand back an artifact. None of that may happen on the egui render thread (ADR-135 §12.2,
//! [ADR-098], [ADR-134]), so the file is read under a byte cap, decoded and verified on a worker,
//! and only this bounded projection crosses into UI state. Repaint code indexes these vectors.
//!
//! The projection reports what the engine sealed and nothing else. It derives no observation,
//! computes no metric and labels no verdict — the per-gate `StageVerdict` and the overall `passed`
//! flag are read straight off the verified artifact, and a registry value the engine left undefined
//! is displayed as undefined rather than given a number.

use std::io::Read;
use std::path::Path;

use typhoon_engine::core::strategy_optimization::{
    MAX_ARTIFACT_BYTES, SampleRole, StageEvidence, StageVerdict,
};
use typhoon_engine::core::strategy_problem_recognition::{
    CalendarGranularity, ConcentrationFamily, ProblemRecognitionArtifact, ProblemRecognitionPolicy,
    ReportProblemObservations,
};
use typhoon_engine::core::strategy_retest::ExecutedPartition;

use super::*;

/// A verified artifact seals exactly the current gate set, so this cap is never reached today. It
/// keeps the projection bounded if a later schema seals more stages, and the overflow is counted
/// and shown rather than silently dropped.
pub(crate) const MAX_PROBLEM_STAGE_ROWS: usize = 32;
/// Display bound for one sealed reason string. The artifact as a whole is capped at
/// `MAX_ARTIFACT_BYTES`, which still leaves room for a reason longer than a panel row.
pub(crate) const MAX_REASON_CHARS: usize = 240;
/// Display bound for executed OOS partitions. The engine's own ceiling on study windows is not
/// public, so this is a native cap; anything beyond it is counted and shown, never dropped quietly.
pub(crate) const MAX_OOS_PARTITION_ROWS: usize = 32;

/// One sealed gate outcome, carried through unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProblemStageRow {
    pub(crate) stage: String,
    pub(crate) verdict: StageVerdict,
    /// The evaluation N the engine bound to this gate (ADR-135 §7.7: every reported result shows it).
    pub(crate) observations_n: usize,
    pub(crate) reason: String,
    pub(crate) reason_truncated: bool,
}

/// One measured observation beside the policy bound it was judged against. Deliberately carries no
/// verdict of its own: the gate outcomes live in `ProblemStageRow`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProblemObservationRow {
    pub(crate) label: &'static str,
    pub(crate) measured: String,
    /// The policy bound this measurement faced, or `—` for a context-only row.
    pub(crate) bound: String,
}

/// Provenance for one of the three sealed studies the verdict was derived from. Composed on the
/// worker into display-ready strings: the decoded study itself, and the compressed bytes it came
/// from, are dropped before the view crosses into UI state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProblemSourceRow {
    pub(crate) study: &'static str,
    pub(crate) artifact_id: String,
    /// The identities this study binds — candidate/strategy, dataset, metric, config.
    pub(crate) binds: String,
    /// The evaluation scope it covers, which is what bounds what this verdict can speak for.
    pub(crate) scope: String,
    /// Set only if the sealed bytes failed to decode here; the other fields are then empty.
    pub(crate) error: Option<String>,
}

/// One executed partition of the sealed OOS scheme, in the order the scheme sealed it. Ranges are
/// half-open exactly as the engine sealed them; the bars themselves are never carried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProblemPartitionRow {
    pub(crate) role: &'static str,
    /// Inclusive start of the sealed half-open range.
    pub(crate) start: usize,
    /// Exclusive end of the sealed half-open range.
    pub(crate) end: usize,
    pub(crate) bars: usize,
    /// The sealed score, or `undefined` when the engine sealed a non-finite one.
    pub(crate) score: String,
    pub(crate) run_id: String,
    pub(crate) report_id: String,
}

/// Everything the panel is allowed to draw, prepared once on the worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProblemRecognitionView {
    pub(crate) artifact_id: String,
    pub(crate) strategy_id: String,
    pub(crate) source_dataset_id: String,
    pub(crate) metric_id: String,
    /// The engine's own sealed verdict over this gate set for this candidate.
    pub(crate) passed: bool,
    pub(crate) stages: Vec<ProblemStageRow>,
    pub(crate) omitted_stages: usize,
    pub(crate) observations: Vec<ProblemObservationRow>,
    /// Exactly one row per sealed source study, in the order the engine derives from them.
    pub(crate) sources: Vec<ProblemSourceRow>,
    /// Executed partitions of the sealed OOS scheme; empty when that source did not decode.
    pub(crate) partitions: Vec<ProblemPartitionRow>,
    pub(crate) omitted_partitions: usize,
}

impl ProblemRecognitionView {
    /// Project an artifact that `from_json_slice`/`verify` has already replayed.
    pub(crate) fn from_verified(artifact: &ProblemRecognitionArtifact) -> Self {
        let (stages, omitted_stages) = project_stages(artifact.stages());
        let ProjectedSources {
            rows: sources,
            partitions,
            omitted_partitions,
        } = project_sources(artifact);
        Self {
            artifact_id: artifact.artifact_id().to_string(),
            strategy_id: artifact.strategy_id().to_string(),
            source_dataset_id: artifact.source_dataset_id().to_string(),
            metric_id: artifact.metric_id().to_string(),
            passed: artifact.passed(),
            stages,
            omitted_stages,
            observations: project_observations(&artifact.observations(), &artifact.policy()),
            sources,
            partitions,
            omitted_partitions,
        }
    }

    pub(crate) fn failed_stage_count(&self) -> usize {
        self.stages
            .iter()
            .filter(|stage| stage.verdict == StageVerdict::Fail)
            .count()
    }
}

/// Request-identity state for the off-thread load. The artifact is standalone — it binds its own
/// dataset/strategy/metric identities and no chart timeline — so request generation is the whole
/// staleness axis: a cancelled or superseded load's late completion is dropped.
#[derive(Clone, Debug, Default)]
pub(crate) struct ProblemLoadState {
    generation: u64,
    active: Option<u64>,
}

impl ProblemLoadState {
    pub(crate) fn begin_request(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.active = Some(self.generation);
        self.generation
    }

    pub(crate) fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.active = None;
    }

    /// Accept a completion only while it is still the outstanding request.
    pub(crate) fn accept(&mut self, request: u64) -> bool {
        if self.active != Some(request) {
            return false;
        }
        self.active = None;
        true
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.active.is_some()
    }
}

/// Worker-side entry point: bounded read, then the verifying decode, then the projection.
pub(crate) fn load_problem_recognition(path: &Path) -> Result<ProblemRecognitionView, String> {
    let bytes = read_bounded(path, MAX_ARTIFACT_BYTES)?;
    // This is the expensive call the render thread must never make: it decompresses the sealed
    // cross-check, OOS and significance studies and replays the whole gate set before returning.
    let artifact =
        ProblemRecognitionArtifact::from_json_slice(&bytes).map_err(|error| error.to_string())?;
    Ok(ProblemRecognitionView::from_verified(&artifact))
}

fn project_stages(stages: &[StageEvidence]) -> (Vec<ProblemStageRow>, usize) {
    let omitted = stages.len().saturating_sub(MAX_PROBLEM_STAGE_ROWS);
    let rows = stages
        .iter()
        .take(MAX_PROBLEM_STAGE_ROWS)
        .map(|stage| {
            let (reason, reason_truncated) = clamp_reason(&stage.reason);
            ProblemStageRow {
                stage: stage.stage.clone(),
                verdict: stage.verdict,
                observations_n: stage.observations_n,
                reason,
                reason_truncated,
            }
        })
        .collect();
    (rows, omitted)
}

/// Decode the three sealed source studies and reduce each to one provenance row.
///
/// This is the second decompression pass, and it stays here on the worker for the same reason the
/// first one does: `source_cross_check`/`source_oos`/`source_significance` each zstd-inflate an
/// embedded study before returning it. Each decoded study is borrowed only long enough to copy the
/// identities and counts out, then dropped — nothing decoded and no compressed byte reaches the
/// view. `verify` already proved all three decode, so the `Err` arms are fail-closed defence, not
/// an expected path.
fn project_sources(artifact: &ProblemRecognitionArtifact) -> ProjectedSources {
    let cross = match artifact.source_cross_check() {
        Ok(cross) => cross_check_row(
            cross.artifact_id(),
            cross.strategy_id(),
            cross.source_dataset_id(),
            cross.metric_id(),
            cross.evaluations_n(),
            cross.checks().len(),
        ),
        Err(error) => unreadable_source_row(CROSS_CHECK_STUDY, &error.to_string()),
    };
    // The OOS scheme is inflated once and yields both its lineage row and its partition rows
    // before this binding is dropped; nothing downstream decodes it a second time.
    let (oos, partitions, omitted_partitions) = match artifact.source_oos() {
        Ok(oos) => {
            let row = oos_row(
                oos.artifact_id(),
                oos.candidate_id(),
                oos.source_dataset_id(),
                oos.metric_id(),
                oos.config_id(),
                oos.source_range().len(),
                oos.executed_partitions().len(),
            );
            let (partitions, omitted) = project_partitions(oos.executed_partitions());
            (row, partitions, omitted)
        }
        Err(error) => (
            unreadable_source_row(OOS_STUDY, &error.to_string()),
            Vec::new(),
            0,
        ),
    };
    let significance = match artifact.source_significance() {
        Ok(significance) => significance_row(
            significance.artifact_id(),
            significance.source_dataset_id(),
            significance.metric_id(),
            significance.evaluations_n(),
            significance.candidates().len(),
        ),
        Err(error) => unreadable_source_row(SIGNIFICANCE_STUDY, &error.to_string()),
    };
    ProjectedSources {
        rows: vec![cross, oos, significance],
        partitions,
        omitted_partitions,
    }
}

/// What one pass over the sealed sources yields, so no study is decoded twice.
struct ProjectedSources {
    rows: Vec<ProblemSourceRow>,
    partitions: Vec<ProblemPartitionRow>,
    omitted_partitions: usize,
}

/// Project the executed partitions in sealed order, bounded by the native display cap.
fn project_partitions(partitions: &[ExecutedPartition]) -> (Vec<ProblemPartitionRow>, usize) {
    let omitted = partitions.len().saturating_sub(MAX_OOS_PARTITION_ROWS);
    let rows = partitions
        .iter()
        .take(MAX_OOS_PARTITION_ROWS)
        .map(|partition| ProblemPartitionRow {
            role: role_label(partition.role),
            start: partition.range.start,
            end: partition.range.end,
            bars: partition.range.len(),
            score: format_score(partition.score),
            run_id: partition.run_id.clone(),
            report_id: partition.report_id.clone(),
        })
        .collect();
    (rows, omitted)
}

fn role_label(role: SampleRole) -> &'static str {
    match role {
        SampleRole::InSample => "in-sample",
        SampleRole::OutOfSample => "out-of-sample",
        SampleRole::Purged => "purged",
        SampleRole::Embargoed => "embargoed",
    }
}

/// A non-finite sealed score is reported as undefined rather than printed as `NaN`/`inf`.
fn format_score(score: f64) -> String {
    if score.is_finite() {
        format!("{score:.6}")
    } else {
        "undefined".to_string()
    }
}

const CROSS_CHECK_STUDY: &str = "Cross-check (§7.5)";
const OOS_STUDY: &str = "Executed OOS (§7.1)";
const SIGNIFICANCE_STUDY: &str = "Adjusted significance (§7.7)";

fn cross_check_row(
    artifact_id: &str,
    strategy_id: &str,
    dataset_id: &str,
    metric_id: &str,
    evaluations_n: usize,
    checks: usize,
) -> ProblemSourceRow {
    source_row(
        CROSS_CHECK_STUDY,
        artifact_id,
        format!(
            "strategy {} · dataset {} · metric {metric_id}",
            short_id(strategy_id),
            short_id(dataset_id)
        ),
        format!("{evaluations_n} evaluations · {checks} sealed checks"),
    )
}

fn oos_row(
    artifact_id: &str,
    candidate_id: &str,
    dataset_id: &str,
    metric_id: &str,
    config_id: &str,
    bars: usize,
    partitions: usize,
) -> ProblemSourceRow {
    source_row(
        OOS_STUDY,
        artifact_id,
        format!(
            "candidate {} · dataset {} · config {} · metric {metric_id}",
            short_id(candidate_id),
            short_id(dataset_id),
            short_id(config_id)
        ),
        format!("{bars} bars · {partitions} executed partitions"),
    )
}

fn significance_row(
    artifact_id: &str,
    dataset_id: &str,
    metric_id: &str,
    evaluations_n: usize,
    candidates: usize,
) -> ProblemSourceRow {
    source_row(
        SIGNIFICANCE_STUDY,
        artifact_id,
        format!("dataset {} · metric {metric_id}", short_id(dataset_id)),
        format!("{evaluations_n} evaluations · {candidates} candidates"),
    )
}

fn source_row(
    study: &'static str,
    artifact_id: &str,
    binds: String,
    scope: String,
) -> ProblemSourceRow {
    ProblemSourceRow {
        study,
        artifact_id: artifact_id.to_string(),
        binds,
        scope,
        error: None,
    }
}

fn unreadable_source_row(study: &'static str, error: &str) -> ProblemSourceRow {
    // Reuses the sealed-reason display bound so one long engine error cannot unbound a row.
    let (error, _) = clamp_reason(error);
    ProblemSourceRow {
        study,
        artifact_id: String::new(),
        binds: String::new(),
        scope: String::new(),
        error: Some(error),
    }
}

fn clamp_reason(reason: &str) -> (String, bool) {
    if reason.chars().count() <= MAX_REASON_CHARS {
        return (reason.to_string(), false);
    }
    (reason.chars().take(MAX_REASON_CHARS).collect(), true)
}

fn project_observations(
    observations: &ReportProblemObservations,
    policy: &ProblemRecognitionPolicy,
) -> Vec<ProblemObservationRow> {
    let edge = observations.edge_concentration;
    let absurd = observations.absurd_metrics;
    let step = observations.parameter_step;
    vec![
        row(
            "Trades",
            observations.trade_count.to_string(),
            format!("min {}", policy.minimum_trades),
        ),
        row(
            "Top-trade PnL share",
            format_bps(observations.top_trade_share_bps),
            format!("max {}", format_bps(policy.maximum_top_trade_share_bps)),
        ),
        row(
            "Time in market",
            format_bps(observations.time_in_market_bps),
            format!("max {}", format_bps(policy.maximum_time_in_market_bps)),
        ),
        row(
            "Sample-boundary trade share",
            format_bps(observations.boundary_trade_share_bps),
            format!(
                "max {} (edge band {})",
                format_bps(policy.maximum_boundary_trade_share_bps),
                format_bps(policy.boundary_width_bps)
            ),
        ),
        row(
            "Retention at 2x costs",
            format_bps(observations.cost_2x_ratio_bps),
            format!("min {}", format_bps(policy.minimum_cost_2x_ratio_bps)),
        ),
        row(
            "Retention at 3x costs",
            format_bps(observations.cost_3x_ratio_bps),
            format!("min {}", format_bps(policy.minimum_cost_3x_ratio_bps)),
        ),
        row(
            "OOS / IS retention",
            format_bps(observations.oos_is_ratio_bps),
            format!("min {}", format_bps(policy.minimum_oos_is_ratio_bps)),
        ),
        row(
            "Worst edge concentration",
            match edge.worst {
                Some((family, share_bps)) => {
                    format!("{} {}", family_label(family), format_bps(share_bps))
                }
                None => "no evaluable family".to_string(),
            },
            format!("max {}", format_bps(policy.maximum_edge_concentration_bps)),
        ),
        row(
            "Calendar concentration",
            format!(
                "{} over {} {} periods",
                format_optional_bps(edge.calendar_share_bps),
                edge.calendar_periods,
                granularity_label(edge.calendar_granularity)
            ),
            CONTEXT_ONLY.to_string(),
        ),
        row(
            "Symbol concentration",
            format!(
                "{} over {} symbols",
                format_optional_bps(edge.symbol_share_bps),
                edge.symbols
            ),
            CONTEXT_ONLY.to_string(),
        ),
        row(
            "Side concentration",
            format!(
                "{} over {} sides",
                format_optional_bps(edge.side_share_bps),
                edge.sides
            ),
            CONTEXT_ONLY.to_string(),
        ),
        row(
            "|Sharpe ratio|",
            format_optional_bps(absurd.absolute_sharpe_bps),
            format!("max {}", format_bps(policy.maximum_absolute_sharpe_bps)),
        ),
        row(
            "Max drawdown",
            format_optional_bps(absurd.max_drawdown_bps),
            format!("min {}", format_bps(policy.minimum_max_drawdown_bps)),
        ),
        row(
            "Profit factor at sentinel",
            if absurd.profit_factor_at_sentinel {
                "yes".to_string()
            } else {
                "no".to_string()
            },
            "no losing trades is the sentinel".to_string(),
        ),
        row(
            "Worst +/-1 parameter step retention",
            format!(
                "{} over {} sealed neighbours",
                format_bps(step.worst_step_ratio_bps),
                step.steps_n
            ),
            format!(
                "min {}",
                format_bps(policy.minimum_parameter_step_ratio_bps)
            ),
        ),
    ]
}

const CONTEXT_ONLY: &str = "—";

fn row(label: &'static str, measured: String, bound: String) -> ProblemObservationRow {
    ProblemObservationRow {
        label,
        measured,
        bound,
    }
}

fn format_bps(value: u32) -> String {
    format!("{:.2}% ({value} bps)", f64::from(value) / 100.0)
}

/// A registry value the engine left undefined stays undefined here; it is never shown as zero.
fn format_optional_bps(value: Option<u32>) -> String {
    match value {
        Some(value) => format_bps(value),
        None => "undefined".to_string(),
    }
}

fn family_label(family: ConcentrationFamily) -> &'static str {
    match family {
        ConcentrationFamily::CalendarPeriod => "calendar-period",
        ConcentrationFamily::Symbol => "symbol",
        ConcentrationFamily::Side => "side",
    }
}

fn granularity_label(granularity: CalendarGranularity) -> &'static str {
    match granularity {
        CalendarGranularity::Annual => "annual",
        CalendarGranularity::Monthly => "monthly",
        CalendarGranularity::Weekly => "weekly",
        CalendarGranularity::Daily => "daily",
    }
}

fn short_id(identity: &str) -> &str {
    identity.get(..8).unwrap_or(identity)
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

/// Read-only. Every value drawn here was prepared and verified on the worker.
pub(crate) fn render_problem_recognition(ui: &mut egui::Ui, view: &ProblemRecognitionView) {
    let failed = view.failed_stage_count();
    ui.horizontal_wrapped(|ui| {
        ui.label("Sealed gate verdict:");
        let (text, color) = if view.passed {
            ("PASS", UP)
        } else {
            ("FAIL", DOWN)
        };
        ui.label(egui::RichText::new(text).color(color).strong());
        ui.label(
            egui::RichText::new(format!(
                "{failed} of {} gates failed",
                view.stages.len() + view.omitted_stages
            ))
            .small(),
        );
    });
    ui.small(format!(
        "artifact {} · strategy {} · dataset {} · metric {}",
        short_id(&view.artifact_id),
        short_id(&view.strategy_id),
        short_id(&view.source_dataset_id),
        view.metric_id
    ));
    ui.small(
        "The engine sealed this verdict over one candidate's §7.6 gate set; it is not a milestone or acceptance decision.",
    );
    if view.omitted_stages > 0 {
        ui.label(
            egui::RichText::new(format!(
                "{} further sealed gates are not shown (display cap {MAX_PROBLEM_STAGE_ROWS})",
                view.omitted_stages
            ))
            .color(egui::Color32::from_rgb(255, 200, 50))
            .small(),
        );
    }
    ui.add_space(4.0);
    egui::Grid::new("problem_recognition_stages")
        .striped(true)
        .num_columns(4)
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Gate").strong());
            ui.label(egui::RichText::new("Verdict").strong());
            ui.label(egui::RichText::new("N").strong());
            ui.label(egui::RichText::new("Sealed reason").strong());
            ui.end_row();
            for stage in &view.stages {
                ui.label(&stage.stage);
                let (text, color) = match stage.verdict {
                    StageVerdict::Pass => ("pass", UP),
                    StageVerdict::Fail => ("fail", DOWN),
                };
                ui.label(egui::RichText::new(text).color(color));
                ui.label(stage.observations_n.to_string());
                let reason = if stage.reason_truncated {
                    format!("{}…", stage.reason)
                } else {
                    stage.reason.clone()
                };
                ui.label(egui::RichText::new(reason).small());
                ui.end_row();
            }
        });
    ui.add_space(4.0);
    ui.collapsing("Sealed evidence this verdict was derived from", |ui| {
        ui.small(
            "Identities and evaluation scope only, decoded on the worker. The verdict can speak for no more than these studies cover.",
        );
        egui::Grid::new("problem_recognition_sources")
            .striped(true)
            .num_columns(4)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Study").strong());
                ui.label(egui::RichText::new("Artifact").strong());
                ui.label(egui::RichText::new("Binds").strong());
                ui.label(egui::RichText::new("Scope").strong());
                ui.end_row();
                for source in &view.sources {
                    ui.label(source.study);
                    match source.error.as_deref() {
                        Some(error) => {
                            ui.label(egui::RichText::new("unreadable").color(DOWN));
                            ui.label(egui::RichText::new(error).color(DOWN).small());
                            ui.label("");
                        }
                        None => {
                            ui.label(
                                egui::RichText::new(short_id(&source.artifact_id)).monospace(),
                            );
                            ui.label(egui::RichText::new(&source.binds).small());
                            ui.label(egui::RichText::new(&source.scope).small());
                        }
                    }
                    ui.end_row();
                }
            });
        if !view.partitions.is_empty() {
            ui.collapsing("Executed OOS partitions", |ui| {
                if view.omitted_partitions > 0 {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} further sealed partitions are not shown (display cap {MAX_OOS_PARTITION_ROWS})",
                            view.omitted_partitions
                        ))
                        .color(egui::Color32::from_rgb(255, 200, 50))
                        .small(),
                    );
                }
                egui::Grid::new("problem_recognition_partitions")
                    .striped(true)
                    .num_columns(5)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Role").strong());
                        ui.label(egui::RichText::new("Bars [start, end)").strong());
                        ui.label(egui::RichText::new("Score").strong());
                        ui.label(egui::RichText::new("Run").strong());
                        ui.label(egui::RichText::new("Report").strong());
                        ui.end_row();
                        for partition in &view.partitions {
                            ui.label(partition.role);
                            ui.label(
                                egui::RichText::new(format!(
                                    "[{}, {}) · {}",
                                    partition.start, partition.end, partition.bars
                                ))
                                .small(),
                            );
                            ui.label(egui::RichText::new(&partition.score).monospace());
                            ui.label(
                                egui::RichText::new(short_id(&partition.run_id)).monospace(),
                            );
                            ui.label(
                                egui::RichText::new(short_id(&partition.report_id)).monospace(),
                            );
                            ui.end_row();
                        }
                    });
            });
        }
    });
    ui.add_space(4.0);
    ui.collapsing("Sealed observations and the bounds they faced", |ui| {
        egui::Grid::new("problem_recognition_observations")
            .striped(true)
            .num_columns(3)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Observation").strong());
                ui.label(egui::RichText::new("Measured").strong());
                ui.label(egui::RichText::new("Policy bound").strong());
                ui.end_row();
                for observation in &view.observations {
                    ui.label(observation.label);
                    ui.label(egui::RichText::new(&observation.measured).monospace());
                    ui.label(egui::RichText::new(&observation.bound).small());
                    ui.end_row();
                }
            });
    });
}

#[cfg(test)]
mod tests;
