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
    StrategyExecutionConfig, StrategyIr, StrategyIrError, StrategyRunManifest,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct RunDatasetInput<'a> {
    pub input_id: &'a str,
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
    DatasetIdMismatch {
        input_id: String,
        expected: String,
        actual: String,
    },
    InvalidDataset {
        input_id: String,
        source: DatasetError,
    },
    MixedAdjustmentPolicy {
        input_id: String,
        expected: AdjustmentPolicy,
        actual: AdjustmentPolicy,
    },
    MissingInterventionLog,
    UnexpectedInterventionLog,
    InterventionLogIdMismatch {
        expected: String,
        actual: String,
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
    intervention_log: Option<&'a InterventionLog>,
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

    pub fn intervention_log(&self) -> Option<&'a InterventionLog> {
        self.intervention_log
    }
}

impl RunDatasetInput<'_> {
    pub fn input_id(&self) -> &str {
        self.input_id
    }
}

pub fn assemble_verified_run<'a>(
    strategy: &'a StrategyIr,
    config: &'a StrategyExecutionConfig,
    manifest: &'a StrategyRunManifest,
    datasets: &[RunDatasetInput<'a>],
) -> Result<VerifiedRun<'a>, RunAssemblyError> {
    assemble_verified_run_with_intervention(strategy, config, manifest, datasets, None)
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

    Ok(VerifiedRun {
        strategy,
        config,
        manifest,
        datasets: resolved,
        intervention_log,
    })
}
