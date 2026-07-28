//! Resolution boundary for a deterministic strategy run.
//!
//! A shape-valid run manifest is not enough to execute: every content address
//! must resolve to the exact sealed artifact and each dataset manifest must be
//! verified against the supplied immutable bars. This module performs that
//! cross-artifact validation once and returns a type that cannot be constructed
//! with unresolved bindings through the public API.

use crate::broker::alpaca::Bar;
use crate::core::strategy_dataset::{AdjustmentPolicy, DatasetError, DatasetManifest};
use crate::core::strategy_intervention::InterventionLog;
use crate::core::strategy_ir::{
    FidelityLevel, RepaintAcknowledgement, StrategyExecutionConfig, StrategyIr, StrategyIrError,
    StrategyRunManifest,
};
use crate::core::strategy_repaint::{RepaintQaArtifact, RepaintQaArtifactError};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct RunDatasetInput<'a> {
    pub input_id: &'a str,
    pub manifest: &'a DatasetManifest,
    pub bars: &'a [Bar],
}

/// The sealed finer-timeframe record bound to one named parent dataset input.
/// `parent_input_id` is a semantic join key, not another strategy input.
#[derive(Debug, Clone, Copy)]
pub struct RunSubBarDatasetInput<'a> {
    pub parent_input_id: &'a str,
    pub manifest: &'a DatasetManifest,
    pub bars: &'a [Bar],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunAssemblyError {
    InvalidStrategy(StrategyIrError),
    InvalidConfig(StrategyIrError),
    InvalidRunManifest(StrategyIrError),
    StrategyIdMismatch {
        expected: String,
        actual: String,
    },
    ConfigIdMismatch {
        expected: String,
        actual: String,
    },
    DuplicateDatasetInput {
        input_id: String,
    },
    MissingDatasetInput {
        input_id: String,
    },
    UnexpectedDatasetInput {
        input_id: String,
    },
    DuplicateSubBarDatasetInput {
        parent_input_id: String,
    },
    MissingSubBarDatasetInput {
        parent_input_id: String,
    },
    UnexpectedSubBarDatasetInput {
        parent_input_id: String,
    },
    DatasetIdMismatch {
        input_id: String,
        expected: String,
        actual: String,
    },
    InvalidDataset {
        input_id: String,
        source: DatasetError,
    },
    SubBarDatasetIdMismatch {
        parent_input_id: String,
        expected: String,
        actual: String,
    },
    InvalidSubBarDataset {
        parent_input_id: String,
        source: DatasetError,
    },
    SubBarFidelityMismatch {
        detail: String,
    },
    SubBarSymbolMismatch {
        parent_input_id: String,
        expected: String,
        actual: String,
    },
    SubBarAdjustmentMismatch {
        parent_input_id: String,
        expected: AdjustmentPolicy,
        actual: AdjustmentPolicy,
    },
    SubBarCalendarMismatch {
        parent_input_id: String,
    },

    UnsupportedSubBarTimeframe {
        parent_input_id: String,
        timeframe: String,
    },
    SubBarTimeframeMismatch {
        parent_input_id: String,
        expected_seconds: u32,
        actual_seconds: u64,
    },
    SubBarTimeframeNotFiner {
        parent_input_id: String,
        parent_timeframe: String,
        sub_bar_timeframe: String,
    },
    MixedAdjustmentPolicy {
        input_id: String,
        expected: AdjustmentPolicy,
        actual: AdjustmentPolicy,
    },
    CorporateActionAdjustmentConflict {
        source: crate::core::strategy_corporate::CorporateActionError,
    },
    MissingInterventionLog,
    UnexpectedInterventionLog,
    InterventionLogIdMismatch {
        expected: String,
        actual: String,
    },
    DuplicateRepaintQaArtifact {
        indicator_id: String,
    },
    MissingRepaintQaArtifact {
        indicator_id: String,
    },
    UnexpectedRepaintQaArtifact {
        indicator_id: String,
    },
    RepaintQaArtifactIdMismatch {
        indicator_id: String,
        expected: String,
        actual: String,
    },
    InvalidRepaintQaArtifact {
        indicator_id: String,
        source: RepaintQaArtifactError,
    },
    RepaintQaDatasetMismatch {
        indicator_id: String,
    },
    RepaintQaAcknowledgementMismatch {
        indicator_id: String,
    },
}

impl std::fmt::Display for RunAssemblyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStrategy(error) => write!(formatter, "invalid strategy artifact: {error}"),
            Self::InvalidConfig(error) => write!(formatter, "invalid execution config: {error}"),
            Self::InvalidRunManifest(error) => write!(formatter, "invalid run manifest: {error}"),
            Self::StrategyIdMismatch { expected, actual } => write!(
                formatter,
                "run manifest strategy id mismatch: expected {expected}, got {actual}"
            ),
            Self::ConfigIdMismatch { expected, actual } => write!(
                formatter,
                "run manifest config id mismatch: expected {expected}, got {actual}"
            ),
            Self::DuplicateDatasetInput { input_id } => {
                write!(formatter, "duplicate dataset input `{input_id}`")
            }
            Self::MissingDatasetInput { input_id } => {
                write!(formatter, "missing dataset input `{input_id}`")
            }
            Self::UnexpectedDatasetInput { input_id } => {
                write!(formatter, "unexpected dataset input `{input_id}`")
            }
            Self::DuplicateSubBarDatasetInput { parent_input_id } => write!(
                formatter,
                "duplicate sub-bar dataset input for parent `{parent_input_id}`"
            ),
            Self::MissingSubBarDatasetInput { parent_input_id } => write!(
                formatter,
                "missing sub-bar dataset input for parent `{parent_input_id}`"
            ),
            Self::UnexpectedSubBarDatasetInput { parent_input_id } => write!(
                formatter,
                "unexpected sub-bar dataset input for parent `{parent_input_id}`"
            ),
            Self::DatasetIdMismatch {
                input_id,
                expected,
                actual,
            } => write!(
                formatter,
                "dataset input `{input_id}` id mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidDataset { input_id, source } => {
                write!(formatter, "dataset input `{input_id}` is invalid: {source}")
            }
            Self::SubBarDatasetIdMismatch {
                parent_input_id,
                expected,
                actual,
            } => write!(
                formatter,
                "sub-bar dataset for parent `{parent_input_id}` id mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidSubBarDataset {
                parent_input_id,
                source,
            } => write!(
                formatter,
                "sub-bar dataset for parent `{parent_input_id}` is invalid: {source}"
            ),
            Self::SubBarFidelityMismatch { detail } => write!(
                formatter,
                "sub-bar dataset bindings contradict execution fidelity: {detail}"
            ),
            Self::SubBarSymbolMismatch {
                parent_input_id,
                expected,
                actual,
            } => write!(
                formatter,
                "sub-bar dataset for parent `{parent_input_id}` has symbol `{actual}`, expected `{expected}`"
            ),
            Self::SubBarAdjustmentMismatch {
                parent_input_id,
                expected,
                actual,
            } => write!(
                formatter,
                "sub-bar dataset for parent `{parent_input_id}` uses adjustment `{}`, expected `{}`",
                actual.wire_id(),
                expected.wire_id()
            ),
            Self::SubBarCalendarMismatch { parent_input_id } => write!(
                formatter,
                "sub-bar dataset for parent `{parent_input_id}` uses a different calendar policy"
            ),

            Self::UnsupportedSubBarTimeframe {
                parent_input_id,
                timeframe,
            } => write!(
                formatter,
                "sub-bar dataset for parent `{parent_input_id}` has unsupported fixed timeframe `{timeframe}`"
            ),
            Self::SubBarTimeframeMismatch {
                parent_input_id,
                expected_seconds,
                actual_seconds,
            } => write!(
                formatter,
                "sub-bar dataset for parent `{parent_input_id}` is {actual_seconds}s, but fidelity binds {expected_seconds}s"
            ),
            Self::SubBarTimeframeNotFiner {
                parent_input_id,
                parent_timeframe,
                sub_bar_timeframe,
            } => write!(
                formatter,
                "sub-bar dataset for parent `{parent_input_id}` timeframe `{sub_bar_timeframe}` is not finer than parent timeframe `{parent_timeframe}`"
            ),
            Self::MixedAdjustmentPolicy {
                input_id,
                expected,
                actual,
            } => write!(
                formatter,
                "dataset input `{input_id}` uses adjustment `{}`, expected `{}`",
                actual.wire_id(),
                expected.wire_id()
            ),
            Self::CorporateActionAdjustmentConflict { source } => write!(
                formatter,
                "corporate-action schedule conflicts with the run's dataset adjustment: {source}"
            ),
            Self::MissingInterventionLog => write!(
                formatter,
                "run manifest binds an intervention log, but none was supplied"
            ),
            Self::UnexpectedInterventionLog => write!(
                formatter,
                "an intervention log was supplied to an automated run"
            ),
            Self::InterventionLogIdMismatch { expected, actual } => write!(
                formatter,
                "run manifest intervention log id mismatch: expected {expected}, got {actual}"
            ),
            Self::DuplicateRepaintQaArtifact { indicator_id } => {
                write!(
                    formatter,
                    "duplicate repaint QA artifact for indicator `{indicator_id}`"
                )
            }
            Self::MissingRepaintQaArtifact { indicator_id } => {
                write!(
                    formatter,
                    "missing repaint QA artifact for indicator `{indicator_id}`"
                )
            }
            Self::UnexpectedRepaintQaArtifact { indicator_id } => {
                write!(
                    formatter,
                    "unexpected repaint QA artifact for indicator `{indicator_id}`"
                )
            }
            Self::RepaintQaArtifactIdMismatch {
                indicator_id,
                expected,
                actual,
            } => write!(
                formatter,
                "repaint QA artifact for indicator `{indicator_id}` id mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidRepaintQaArtifact {
                indicator_id,
                source,
            } => write!(
                formatter,
                "repaint QA artifact for indicator `{indicator_id}` is invalid: {source}"
            ),
            Self::RepaintQaDatasetMismatch { indicator_id } => write!(
                formatter,
                "repaint QA artifact for indicator `{indicator_id}` does not bind a run dataset"
            ),
            Self::RepaintQaAcknowledgementMismatch { indicator_id } => write!(
                formatter,
                "repaint QA acknowledgement for indicator `{indicator_id}` contradicts its report"
            ),
        }
    }
}

impl std::error::Error for RunAssemblyError {}

/// A run whose strategy, execution config, manifest, and datasets have all
/// passed identity and cross-artifact verification.
#[derive(Debug)]
pub struct VerifiedRun<'a> {
    strategy: &'a StrategyIr,
    config: &'a StrategyExecutionConfig,
    manifest: &'a StrategyRunManifest,
    datasets: Vec<RunDatasetInput<'a>>,
    sub_bar_datasets: Vec<RunSubBarDatasetInput<'a>>,
    intervention_log: Option<&'a InterventionLog>,
    repaint_qa_artifacts: Vec<&'a RepaintQaArtifact>,
}

impl<'a> VerifiedRun<'a> {
    pub fn strategy(&self) -> &'a StrategyIr {
        self.strategy
    }

    pub fn config(&self) -> &'a StrategyExecutionConfig {
        self.config
    }

    pub fn manifest(&self) -> &'a StrategyRunManifest {
        self.manifest
    }

    pub fn run_id(&self) -> &str {
        self.manifest.run_id()
    }

    pub fn datasets(&self) -> &[RunDatasetInput<'a>] {
        &self.datasets
    }

    pub fn sub_bar_datasets(&self) -> &[RunSubBarDatasetInput<'a>] {
        &self.sub_bar_datasets
    }

    pub fn intervention_log(&self) -> Option<&'a InterventionLog> {
        self.intervention_log
    }

    pub fn repaint_qa_artifacts(&self) -> &[&'a RepaintQaArtifact] {
        &self.repaint_qa_artifacts
    }
}

impl RunDatasetInput<'_> {
    pub fn input_id(&self) -> &str {
        self.input_id
    }
}

impl RunSubBarDatasetInput<'_> {
    pub fn parent_input_id(&self) -> &str {
        self.parent_input_id
    }
}

pub fn assemble_verified_run<'a>(
    strategy: &'a StrategyIr,
    config: &'a StrategyExecutionConfig,
    manifest: &'a StrategyRunManifest,
    datasets: &[RunDatasetInput<'a>],
) -> Result<VerifiedRun<'a>, RunAssemblyError> {
    assemble_verified_run_with_all_artifacts(strategy, config, manifest, datasets, &[], None, &[])
}

/// Resolve an automated run together with every bound finer-timeframe record.
pub fn assemble_verified_run_with_sub_bars<'a>(
    strategy: &'a StrategyIr,
    config: &'a StrategyExecutionConfig,
    manifest: &'a StrategyRunManifest,
    datasets: &[RunDatasetInput<'a>],
    sub_bar_datasets: &[RunSubBarDatasetInput<'a>],
) -> Result<VerifiedRun<'a>, RunAssemblyError> {
    assemble_verified_run_with_all_artifacts(
        strategy,
        config,
        manifest,
        datasets,
        sub_bar_datasets,
        None,
        &[],
    )
}

/// Resolves a run and, for a hybrid manifest, proves that the supplied sealed
/// intervention log is exactly the artifact included in the run identity.
pub fn assemble_verified_run_with_intervention<'a>(
    strategy: &'a StrategyIr,
    config: &'a StrategyExecutionConfig,
    manifest: &'a StrategyRunManifest,
    datasets: &[RunDatasetInput<'a>],
    intervention_log: Option<&'a InterventionLog>,
) -> Result<VerifiedRun<'a>, RunAssemblyError> {
    assemble_verified_run_with_all_artifacts(
        strategy,
        config,
        manifest,
        datasets,
        &[],
        intervention_log,
        &[],
    )
}

/// Resolves every optional identity-bearing artifact required by a run.
pub fn assemble_verified_run_with_artifacts<'a>(
    strategy: &'a StrategyIr,
    config: &'a StrategyExecutionConfig,
    manifest: &'a StrategyRunManifest,
    datasets: &[RunDatasetInput<'a>],
    intervention_log: Option<&'a InterventionLog>,
    repaint_qa_artifacts: &[&'a RepaintQaArtifact],
) -> Result<VerifiedRun<'a>, RunAssemblyError> {
    assemble_verified_run_with_all_artifacts(
        strategy,
        config,
        manifest,
        datasets,
        &[],
        intervention_log,
        repaint_qa_artifacts,
    )
}

/// Resolves every identity-bearing artifact required by a run, including the
/// immutable finer-timeframe records consumed by level-3 execution.
pub fn assemble_verified_run_with_all_artifacts<'a>(
    strategy: &'a StrategyIr,
    config: &'a StrategyExecutionConfig,
    manifest: &'a StrategyRunManifest,
    datasets: &[RunDatasetInput<'a>],
    sub_bar_datasets: &[RunSubBarDatasetInput<'a>],
    intervention_log: Option<&'a InterventionLog>,
    repaint_qa_artifacts: &[&'a RepaintQaArtifact],
) -> Result<VerifiedRun<'a>, RunAssemblyError> {
    strategy
        .verify()
        .map_err(RunAssemblyError::InvalidStrategy)?;
    config.verify().map_err(RunAssemblyError::InvalidConfig)?;
    manifest
        .verify()
        .map_err(RunAssemblyError::InvalidRunManifest)?;

    let binding = manifest.binding();
    if binding.strategy_id != strategy.strategy_id() {
        return Err(RunAssemblyError::StrategyIdMismatch {
            expected: binding.strategy_id.clone(),
            actual: strategy.strategy_id().to_string(),
        });
    }
    if binding.config_id != config.config_id() {
        return Err(RunAssemblyError::ConfigIdMismatch {
            expected: binding.config_id.clone(),
            actual: config.config_id().to_string(),
        });
    }
    match (&binding.intervention_log_id, intervention_log) {
        (Some(_), None) => return Err(RunAssemblyError::MissingInterventionLog),
        (None, Some(_)) => return Err(RunAssemblyError::UnexpectedInterventionLog),
        (Some(expected), Some(log)) if expected != log.log_id() => {
            return Err(RunAssemblyError::InterventionLogIdMismatch {
                expected: expected.clone(),
                actual: log.log_id().to_string(),
            });
        }
        _ => {}
    }

    let mut supplied_qa = BTreeMap::new();
    for artifact in repaint_qa_artifacts {
        let indicator_id = artifact.indicator_id();
        if supplied_qa.insert(indicator_id, *artifact).is_some() {
            return Err(RunAssemblyError::DuplicateRepaintQaArtifact {
                indicator_id: indicator_id.to_string(),
            });
        }
    }
    let mut resolved_qa = Vec::with_capacity(binding.repaint_qa.len());
    for expected in &binding.repaint_qa {
        let Some(artifact) = supplied_qa.remove(expected.indicator_id.as_str()) else {
            return Err(RunAssemblyError::MissingRepaintQaArtifact {
                indicator_id: expected.indicator_id.clone(),
            });
        };
        artifact
            .verify()
            .map_err(|source| RunAssemblyError::InvalidRepaintQaArtifact {
                indicator_id: expected.indicator_id.clone(),
                source,
            })?;
        if expected.artifact_id != artifact.artifact_id() {
            return Err(RunAssemblyError::RepaintQaArtifactIdMismatch {
                indicator_id: expected.indicator_id.clone(),
                expected: expected.artifact_id.clone(),
                actual: artifact.artifact_id().to_string(),
            });
        }
        if !binding.datasets.iter().any(|dataset| {
            dataset.input_id == artifact.dataset_input_id()
                && dataset.dataset_id == artifact.dataset_id()
        }) {
            return Err(RunAssemblyError::RepaintQaDatasetMismatch {
                indicator_id: expected.indicator_id.clone(),
            });
        }
        let acknowledgement_matches = match &expected.acknowledgement {
            RepaintAcknowledgement::Clean => artifact.report().is_clean(),
            RepaintAcknowledgement::WarningAcknowledged { .. } => !artifact.report().is_clean(),
        };
        if !acknowledgement_matches {
            return Err(RunAssemblyError::RepaintQaAcknowledgementMismatch {
                indicator_id: expected.indicator_id.clone(),
            });
        }
        resolved_qa.push(artifact);
    }
    if let Some((indicator_id, _)) = supplied_qa.into_iter().next() {
        return Err(RunAssemblyError::UnexpectedRepaintQaArtifact {
            indicator_id: indicator_id.to_string(),
        });
    }

    let mut supplied = BTreeMap::new();
    for dataset in datasets {
        if supplied.insert(dataset.input_id, *dataset).is_some() {
            return Err(RunAssemblyError::DuplicateDatasetInput {
                input_id: dataset.input_id.to_string(),
            });
        }
    }

    let mut adjustment = None;
    let mut resolved = Vec::with_capacity(binding.datasets.len());
    for expected in &binding.datasets {
        let Some(dataset) = supplied.remove(expected.input_id.as_str()) else {
            return Err(RunAssemblyError::MissingDatasetInput {
                input_id: expected.input_id.clone(),
            });
        };
        dataset.manifest.verify(dataset.bars).map_err(|source| {
            RunAssemblyError::InvalidDataset {
                input_id: expected.input_id.clone(),
                source,
            }
        })?;
        if dataset.manifest.dataset_id != expected.dataset_id {
            return Err(RunAssemblyError::DatasetIdMismatch {
                input_id: expected.input_id.clone(),
                expected: expected.dataset_id.clone(),
                actual: dataset.manifest.dataset_id.clone(),
            });
        }
        if let Some(expected_policy) = adjustment {
            if dataset.manifest.adjustment != expected_policy {
                return Err(RunAssemblyError::MixedAdjustmentPolicy {
                    input_id: expected.input_id.clone(),
                    expected: expected_policy,
                    actual: dataset.manifest.adjustment,
                });
            }
        } else {
            adjustment = Some(dataset.manifest.adjustment);
        }
        resolved.push(dataset);
    }

    if let Some((input_id, _)) = supplied.into_iter().next() {
        return Err(RunAssemblyError::UnexpectedDatasetInput {
            input_id: input_id.to_string(),
        });
    }

    let sub_bar_seconds = match config.settings().fidelity {
        FidelityLevel::SubBar { sub_bar_seconds } => {
            if binding.sub_bar_datasets.len() != binding.datasets.len() {
                return Err(RunAssemblyError::SubBarFidelityMismatch {
                    detail: format!(
                        "sub-bar fidelity requires exactly one binding for each of {} parent inputs, found {}",
                        binding.datasets.len(),
                        binding.sub_bar_datasets.len()
                    ),
                });
            }
            Some(sub_bar_seconds)
        }
        _ => {
            if !binding.sub_bar_datasets.is_empty() {
                return Err(RunAssemblyError::SubBarFidelityMismatch {
                    detail:
                        "the run binds sub-bar datasets at a fidelity that does not consume them"
                            .to_string(),
                });
            }
            None
        }
    };

    let mut supplied_sub_bars = BTreeMap::new();
    for dataset in sub_bar_datasets {
        if supplied_sub_bars
            .insert(dataset.parent_input_id, *dataset)
            .is_some()
        {
            return Err(RunAssemblyError::DuplicateSubBarDatasetInput {
                parent_input_id: dataset.parent_input_id.to_string(),
            });
        }
    }
    let mut resolved_sub_bars = Vec::with_capacity(binding.sub_bar_datasets.len());
    for expected in &binding.sub_bar_datasets {
        let Some(dataset) = supplied_sub_bars.remove(expected.parent_input_id.as_str()) else {
            return Err(RunAssemblyError::MissingSubBarDatasetInput {
                parent_input_id: expected.parent_input_id.clone(),
            });
        };
        dataset.manifest.verify(dataset.bars).map_err(|source| {
            RunAssemblyError::InvalidSubBarDataset {
                parent_input_id: expected.parent_input_id.clone(),
                source,
            }
        })?;
        if dataset.manifest.dataset_id != expected.dataset_id {
            return Err(RunAssemblyError::SubBarDatasetIdMismatch {
                parent_input_id: expected.parent_input_id.clone(),
                expected: expected.dataset_id.clone(),
                actual: dataset.manifest.dataset_id.clone(),
            });
        }
        let parent = resolved
            .iter()
            .find(|parent| parent.input_id == expected.parent_input_id)
            .expect("run binding validation proved the parent exists");
        if dataset.manifest.symbol != parent.manifest.symbol {
            return Err(RunAssemblyError::SubBarSymbolMismatch {
                parent_input_id: expected.parent_input_id.clone(),
                expected: parent.manifest.symbol.clone(),
                actual: dataset.manifest.symbol.clone(),
            });
        }
        if dataset.manifest.adjustment != parent.manifest.adjustment {
            return Err(RunAssemblyError::SubBarAdjustmentMismatch {
                parent_input_id: expected.parent_input_id.clone(),
                expected: parent.manifest.adjustment,
                actual: dataset.manifest.adjustment,
            });
        }
        if dataset.manifest.calendar != parent.manifest.calendar {
            return Err(RunAssemblyError::SubBarCalendarMismatch {
                parent_input_id: expected.parent_input_id.clone(),
            });
        }
        let actual_seconds =
            fixed_timeframe_seconds(&dataset.manifest.timeframe).ok_or_else(|| {
                RunAssemblyError::UnsupportedSubBarTimeframe {
                    parent_input_id: expected.parent_input_id.clone(),
                    timeframe: dataset.manifest.timeframe.clone(),
                }
            })?;
        let expected_seconds = sub_bar_seconds.expect("bindings require sub-bar fidelity");
        if actual_seconds != u64::from(expected_seconds) {
            return Err(RunAssemblyError::SubBarTimeframeMismatch {
                parent_input_id: expected.parent_input_id.clone(),
                expected_seconds,
                actual_seconds,
            });
        }
        let parent_seconds =
            fixed_timeframe_seconds(&parent.manifest.timeframe).ok_or_else(|| {
                RunAssemblyError::SubBarTimeframeNotFiner {
                    parent_input_id: expected.parent_input_id.clone(),
                    parent_timeframe: parent.manifest.timeframe.clone(),
                    sub_bar_timeframe: dataset.manifest.timeframe.clone(),
                }
            })?;
        if actual_seconds >= parent_seconds {
            return Err(RunAssemblyError::SubBarTimeframeNotFiner {
                parent_input_id: expected.parent_input_id.clone(),
                parent_timeframe: parent.manifest.timeframe.clone(),
                sub_bar_timeframe: dataset.manifest.timeframe.clone(),
            });
        }
        resolved_sub_bars.push(dataset);
    }
    if let Some((parent_input_id, _)) = supplied_sub_bars.into_iter().next() {
        return Err(RunAssemblyError::UnexpectedSubBarDatasetInput {
            parent_input_id: parent_input_id.to_string(),
        });
    }

    // §6.8: the adjustment policy and event schedule are two representations
    // of the same economics. This check belongs at verified assembly, where the
    // identity-bound datasets and execution config are finally both present;
    // checking either artifact alone cannot detect a double-counted split or
    // dividend.
    if let Some(adjustment) = adjustment {
        config
            .settings()
            .corporate_actions
            .check_adjustment_consistency(adjustment)
            .map_err(|source| RunAssemblyError::CorporateActionAdjustmentConflict { source })?;
    }

    Ok(VerifiedRun {
        strategy,
        config,
        manifest,
        datasets: resolved,
        sub_bar_datasets: resolved_sub_bars,
        intervention_log,
        repaint_qa_artifacts: resolved_qa,
    })
}

fn fixed_timeframe_seconds(timeframe: &str) -> Option<u64> {
    let digits = timeframe
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(timeframe.len());
    if digits == 0 {
        return None;
    }
    let count = timeframe[..digits].parse::<u64>().ok()?;
    if count == 0 {
        return None;
    }
    let unit = match &timeframe[digits..] {
        "Min" => 60,
        "Hour" => 3_600,
        "Day" => 86_400,
        "Week" => 604_800,
        _ => return None,
    };
    count.checked_mul(unit)
}
