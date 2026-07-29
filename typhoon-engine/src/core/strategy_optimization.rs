//! Deterministic, bounded ADR-135 M4 optimizer and robustness primitives.
//!
//! This module deliberately owns orchestration and evidence, not simulation. Candidate IRs are
//! sealed through [`StrategyIr`]; callers evaluate them through the existing verified CPU run and
//! sealed-report path. The helpers here never manufacture backtest metrics or invoke the legacy
//! backtester.
//!
//! M4 implements bounded grid, random, local and Latin-hypercube parameter plans plus objective
//! and Pareto evaluation contracts. Adaptive Bayesian TPE/GP is intentionally not mislabeled by a
//! random sampler here: ADR-135 M5 owns its checkpoint/resume search operator (§8.2, §8.6), after
//! this M4 report-observation and objective boundary exists.
//!
//! Evidence enters through one door. [`ReportObservation::from_report`] is the only constructor of
//! an evaluated metric, and it admits a value only after the retest request, the retest result, the
//! sealed report identity, the report's own run manifest and the search-partition lease all agree.
//! [`RobustnessPipeline`] then runs declared stages in a deterministic bounded order, stops on the
//! first failure, and publishes the exact distributions each stage consumed (§5.6, §7.3, §7.7).
//! No stage can name the final holdout: a lease never carries that partition (§7.8).

use crate::core::strategy_ir::{ParamRange, ParamValue, StrategyIr};
use crate::core::strategy_metrics::MetricValue;
use crate::core::strategy_report::StrategyReportArtifact;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};

pub const MAX_TRIAL_BUDGET: usize = 10_000;
pub const MAX_SEARCH_COMBINATIONS: usize = 1_000_000;
pub const MAX_MONTE_CARLO_TRIALS: usize = 10_000;
pub const MAX_ROBUSTNESS_STAGES: usize = 128;
pub const MAX_ARTIFACT_BYTES: usize = 1_048_576;
const ARTIFACT_DOMAIN: &[u8] = b"typhoon.strategy_optimization.robustness.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizationError {
    EmptyDomain { parameter: String },
    UnknownParameter { parameter: String },
    DuplicateDomain { parameter: String },
    TypeMismatch { parameter: String },
    OutOfRange { parameter: String },
    NonFiniteParameter { parameter: String },
    InvalidBudget { found: usize },
    CombinationLimit { found: usize },
    InvalidFold { detail: String },
    HoldoutForbidden,
    InvalidPerturbation,
    InvalidObservation,
    InvalidArtifact(String),
    Worker(String),
}

impl std::fmt::Display for OptimizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "optimization error: {self:?}")
    }
}
impl std::error::Error for OptimizationError {}

#[derive(Debug, Clone, PartialEq)]
pub struct ParameterDomain {
    id: String,
    values: Vec<ParamValue>,
}

impl ParameterDomain {
    pub fn new(
        id: impl Into<String>,
        mut values: Vec<ParamValue>,
    ) -> Result<Self, OptimizationError> {
        let id = id.into();
        if values.is_empty() {
            return Err(OptimizationError::EmptyDomain { parameter: id });
        }
        for value in &mut values {
            canonicalize_value(value, &id)?;
        }
        values.sort_by(|a, b| value_key(a).cmp(&value_key(b)));
        values.dedup_by(|a, b| value_key(a) == value_key(b));
        Ok(Self { id, values })
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn values(&self) -> &[ParamValue] {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchSpace {
    base: StrategyIr,
    domains: Vec<ParameterDomain>,
    combinations: usize,
}

impl SearchSpace {
    pub fn new(
        base: StrategyIr,
        mut domains: Vec<ParameterDomain>,
    ) -> Result<Self, OptimizationError> {
        base.verify()
            .map_err(|error| OptimizationError::InvalidArtifact(error.to_string()))?;
        domains.sort_by(|a, b| a.id.cmp(&b.id));
        for pair in domains.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(OptimizationError::DuplicateDomain {
                    parameter: pair[0].id.clone(),
                });
            }
        }
        let declared: BTreeMap<_, _> = base
            .definition()
            .parameters
            .iter()
            .map(|p| (p.id.as_str(), p))
            .collect();
        let mut combinations = 1usize;
        for domain in &domains {
            let parameter = declared.get(domain.id.as_str()).ok_or_else(|| {
                OptimizationError::UnknownParameter {
                    parameter: domain.id.clone(),
                }
            })?;
            for value in &domain.values {
                validate_assignment(parameter, value)?;
            }
            combinations = combinations
                .checked_mul(domain.values.len())
                .ok_or(OptimizationError::CombinationLimit { found: usize::MAX })?;
            if combinations > MAX_SEARCH_COMBINATIONS {
                return Err(OptimizationError::CombinationLimit {
                    found: combinations,
                });
            }
        }
        if domains.is_empty() {
            return Err(OptimizationError::EmptyDomain {
                parameter: "search_space".into(),
            });
        }
        Ok(Self {
            base,
            domains,
            combinations,
        })
    }
    pub fn combinations(&self) -> usize {
        self.combinations
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMethod {
    Grid,
    Random { seed: u64 },
    LatinHypercube { seed: u64 },
    Local,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub candidate_id: String,
    pub assignments: Vec<(String, ParamValue)>,
    pub strategy: StrategyIr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchBatch {
    pub candidates: Vec<Candidate>,
    pub evaluations_n: usize,
    pub duplicates_skipped: usize,
    pub exhausted_budget: bool,
}

pub fn generate_candidates(
    space: &SearchSpace,
    method: SearchMethod,
    budget: usize,
) -> Result<SearchBatch, OptimizationError> {
    if budget == 0 || budget > MAX_TRIAL_BUDGET {
        return Err(OptimizationError::InvalidBudget { found: budget });
    }
    let target = budget.min(space.combinations);
    let mut ordinals = Vec::with_capacity(target);
    match method {
        SearchMethod::Grid => ordinals.extend(0..target),
        SearchMethod::Random { seed } => {
            let mut rng = SplitMix64(seed);
            let mut seen = BTreeSet::new();
            let attempt_limit = target.saturating_mul(16).max(32);
            for _ in 0..attempt_limit {
                if ordinals.len() == target {
                    break;
                }
                let ordinal = (rng.next() as usize) % space.combinations;
                if seen.insert(ordinal) {
                    ordinals.push(ordinal);
                }
            }
            if ordinals.len() < target {
                for ordinal in 0..space.combinations {
                    if seen.insert(ordinal) {
                        ordinals.push(ordinal);
                    }
                    if ordinals.len() == target {
                        break;
                    }
                }
            }
        }
        SearchMethod::LatinHypercube { seed } => {
            let mut rng = SplitMix64(seed);
            let mut strata_by_domain = Vec::with_capacity(space.domains.len());
            for _ in &space.domains {
                let mut strata: Vec<_> = (0..target).collect();
                for index in (1..strata.len()).rev() {
                    let other = (rng.next() as usize) % (index + 1);
                    strata.swap(index, other);
                }
                strata_by_domain.push(strata);
            }
            let mut seen = BTreeSet::new();
            for sample in 0..target {
                let indices: Vec<_> = space
                    .domains
                    .iter()
                    .zip(&strata_by_domain)
                    .map(|(domain, strata)| strata[sample] * domain.values.len() / target)
                    .collect();
                let ordinal = indices_ordinal(space, &indices);
                if seen.insert(ordinal) {
                    ordinals.push(ordinal);
                }
            }
            // Discrete axes can collapse strata when N exceeds an axis cardinality. Fill
            // deterministically without changing the Latin-hypercube prefix.
            for ordinal in 0..space.combinations {
                if ordinals.len() == target {
                    break;
                }
                if seen.insert(ordinal) {
                    ordinals.push(ordinal);
                }
            }
        }
        SearchMethod::Local => {
            let center = center_indices(space);
            let mut ranked = (0..space.combinations)
                .map(|ordinal| {
                    let indices = ordinal_indices(space, ordinal);
                    let distance: usize = indices
                        .iter()
                        .zip(&center)
                        .map(|(a, b)| a.abs_diff(*b))
                        .sum();
                    (distance, ordinal)
                })
                .collect::<Vec<_>>();
            ranked.sort_unstable();
            ordinals.extend(ranked.into_iter().take(target).map(|(_, ordinal)| ordinal));
        }
    }
    let mut candidates = Vec::with_capacity(target);
    let mut ids = BTreeSet::new();
    let mut duplicates_skipped = 0;
    for ordinal in ordinals {
        let candidate = instantiate(space, ordinal)?;
        if ids.insert(candidate.candidate_id.clone()) {
            candidates.push(candidate);
        } else {
            duplicates_skipped += 1;
        }
    }
    Ok(SearchBatch {
        evaluations_n: candidates.len(),
        exhausted_budget: budget < space.combinations,
        candidates,
        duplicates_skipped,
    })
}

fn instantiate(space: &SearchSpace, ordinal: usize) -> Result<Candidate, OptimizationError> {
    let indices = ordinal_indices(space, ordinal);
    let assignments: Vec<_> = space
        .domains
        .iter()
        .zip(indices)
        .map(|(domain, index)| (domain.id.clone(), domain.values[index].clone()))
        .collect();
    let lookup: BTreeMap<_, _> = assignments.iter().cloned().collect();
    let mut definition = space.base.to_input();
    for parameter in &mut definition.parameters {
        if let Some(value) = lookup.get(&parameter.id) {
            parameter.value = value.clone();
        }
    }
    let strategy = StrategyIr::build(&definition)
        .map_err(|error| OptimizationError::InvalidArtifact(error.to_string()))?;
    Ok(Candidate {
        candidate_id: strategy.strategy_id().to_string(),
        assignments,
        strategy,
    })
}

fn ordinal_indices(space: &SearchSpace, mut ordinal: usize) -> Vec<usize> {
    let mut indices = vec![0; space.domains.len()];
    for index in (0..space.domains.len()).rev() {
        indices[index] = ordinal % space.domains[index].values.len();
        ordinal /= space.domains[index].values.len();
    }
    indices
}

fn indices_ordinal(space: &SearchSpace, indices: &[usize]) -> usize {
    space
        .domains
        .iter()
        .zip(indices)
        .fold(0usize, |ordinal, (domain, index)| {
            ordinal * domain.values.len() + index
        })
}

fn center_indices(space: &SearchSpace) -> Vec<usize> {
    space
        .domains
        .iter()
        .map(|domain| {
            let current = space
                .base
                .definition()
                .parameters
                .iter()
                .find(|p| p.id == domain.id)
                .map(|p| &p.value);
            current
                .and_then(|value| {
                    domain
                        .values
                        .iter()
                        .position(|candidate| value_key(candidate) == value_key(value))
                })
                .unwrap_or(domain.values.len() / 2)
        })
        .collect()
}

fn validate_assignment(
    parameter: &&crate::core::strategy_ir::StrategyParameter,
    value: &ParamValue,
) -> Result<(), OptimizationError> {
    let same_type = matches!(
        (&parameter.value, value),
        (ParamValue::Bool(_), ParamValue::Bool(_))
            | (ParamValue::Int(_), ParamValue::Int(_))
            | (ParamValue::Float(_), ParamValue::Float(_))
            | (ParamValue::Text(_), ParamValue::Text(_))
    );
    if !same_type {
        return Err(OptimizationError::TypeMismatch {
            parameter: parameter.id.clone(),
        });
    }
    let inside = match (&parameter.range, value) {
        (Some(ParamRange::Int { min, max }), ParamValue::Int(value)) => {
            value >= min && value <= max
        }
        (Some(ParamRange::Float { min, max }), ParamValue::Float(value)) => {
            value >= min && value <= max
        }
        (None, _) => value_key(&parameter.value) == value_key(value),
        _ => false,
    };
    if inside {
        Ok(())
    } else {
        Err(OptimizationError::OutOfRange {
            parameter: parameter.id.clone(),
        })
    }
}

fn canonicalize_value(value: &mut ParamValue, parameter: &str) -> Result<(), OptimizationError> {
    if let ParamValue::Float(number) = value {
        if !number.is_finite() {
            return Err(OptimizationError::NonFiniteParameter {
                parameter: parameter.into(),
            });
        }
        if *number == 0.0 {
            *number = 0.0;
        }
    }
    Ok(())
}
fn value_key(value: &ParamValue) -> (u8, Vec<u8>) {
    match value {
        ParamValue::Bool(v) => (0, vec![u8::from(*v)]),
        ParamValue::Int(v) => (1, v.to_be_bytes().to_vec()),
        ParamValue::Float(v) => (
            2,
            (if *v == 0.0 { 0.0 } else { *v })
                .to_bits()
                .to_be_bytes()
                .to_vec(),
        ),
        ParamValue::Text(v) => (3, v.as_bytes().to_vec()),
    }
}

const RETEST_REQUEST_DOMAIN: &[u8] = b"typhoon.strategy_optimization.retest.request.v1";
const RETEST_RESULT_DOMAIN: &[u8] = b"typhoon.strategy_optimization.retest.result.v1";

/// Immutable identity for re-running a stored strategy through the verified simulation/report
/// boundary. The optimizer never accepts bars or manufactures a metric vector itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetestRequest {
    request_id: String,
    strategy_id: String,
    dataset_id: String,
    execution_config_id: String,
    metrics_version: String,
    root_seed: u64,
    stage: StageAccess,
    range_start: usize,
    range_end: usize,
}
impl RetestRequest {
    pub fn seal(
        strategy: &StrategyIr,
        lease: &SearchDataLease,
        execution_config_id: impl Into<String>,
        metrics_version: impl Into<String>,
        root_seed: u64,
    ) -> Result<Self, OptimizationError> {
        strategy
            .verify()
            .map_err(|error| OptimizationError::InvalidArtifact(error.to_string()))?;
        let mut request = Self {
            request_id: String::new(),
            strategy_id: strategy.strategy_id().to_string(),
            dataset_id: lease.dataset_id.clone(),
            execution_config_id: execution_config_id.into(),
            metrics_version: metrics_version.into(),
            root_seed,
            stage: lease.stage,
            range_start: lease.range.start,
            range_end: lease.range.end,
        };
        request.validate()?;
        request.request_id = request.compute_id();
        Ok(request)
    }
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
    pub fn strategy_id(&self) -> &str {
        &self.strategy_id
    }
    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }
    pub fn root_seed(&self) -> u64 {
        self.root_seed
    }
    fn validate(&self) -> Result<(), OptimizationError> {
        if self.strategy_id.trim().is_empty()
            || self.dataset_id.trim().is_empty()
            || self.execution_config_id.trim().is_empty()
            || self.metrics_version.trim().is_empty()
            || !matches!(self.stage, StageAccess::Search | StageAccess::Robustness)
            || self.range_start >= self.range_end
        {
            return Err(OptimizationError::InvalidArtifact(
                "retest identity contains an empty component".into(),
            ));
        }
        Ok(())
    }
    fn compute_id(&self) -> String {
        identity_hex(RETEST_REQUEST_DOMAIN, |hasher| {
            frame(hasher, self.strategy_id.as_bytes());
            frame(hasher, self.dataset_id.as_bytes());
            frame(hasher, self.execution_config_id.as_bytes());
            frame(hasher, self.metrics_version.as_bytes());
            hasher.update(self.root_seed.to_be_bytes());
            hasher.update([match self.stage {
                StageAccess::Search => 0,
                StageAccess::Robustness => 1,
                StageAccess::FinalReview => 2,
            }]);
            hasher.update((self.range_start as u64).to_be_bytes());
            hasher.update((self.range_end as u64).to_be_bytes());
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetestResult {
    result_id: String,
    request_id: String,
    report_id: String,
}
impl RetestResult {
    pub fn seal(
        request: &RetestRequest,
        report_id: impl Into<String>,
    ) -> Result<Self, OptimizationError> {
        request.validate()?;
        if request.request_id != request.compute_id() {
            return Err(OptimizationError::InvalidArtifact(
                "retest request identity mismatch".into(),
            ));
        }
        let mut result = Self {
            result_id: String::new(),
            request_id: request.request_id.clone(),
            report_id: report_id.into(),
        };
        if result.report_id.trim().is_empty() {
            return Err(OptimizationError::InvalidArtifact(
                "empty retest report identity".into(),
            ));
        }
        result.result_id = result.compute_id();
        Ok(result)
    }
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
    pub fn result_id(&self) -> &str {
        &self.result_id
    }
    pub fn report_id(&self) -> &str {
        &self.report_id
    }
    pub fn verify_against(&self, request: &RetestRequest) -> Result<(), OptimizationError> {
        if self.request_id != request.request_id
            || request.request_id != request.compute_id()
            || self.result_id != self.compute_id()
        {
            return Err(OptimizationError::InvalidArtifact(
                "retest result identity mismatch".into(),
            ));
        }
        Ok(())
    }
    fn compute_id(&self) -> String {
        identity_hex(RETEST_RESULT_DOMAIN, |hasher| {
            frame(hasher, self.request_id.as_bytes());
            frame(hasher, self.report_id.as_bytes());
        })
    }
}

fn identity_hex(domain: &[u8], update: impl FnOnce(&mut Sha256)) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    update(&mut hasher);
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fold {
    pub train: Range<usize>,
    pub test: Range<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldPlan {
    folds: Vec<Fold>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkForwardConfig {
    pub train_bars: usize,
    pub test_bars: usize,
    pub step_bars: usize,
    pub purge_bars: usize,
    pub embargo_bars: usize,
    pub anchored: bool,
}

impl FoldPlan {
    pub fn trailing_holdout(
        total: usize,
        test_bars: usize,
        purge_bars: usize,
        embargo_bars: usize,
    ) -> Result<Self, OptimizationError> {
        if test_bars == 0 || test_bars >= total || purge_bars + embargo_bars >= total - test_bars {
            return Err(OptimizationError::InvalidFold {
                detail: "empty train/test after purge and embargo".into(),
            });
        }
        let test_start = total - test_bars;
        Ok(Self {
            folds: vec![Fold {
                train: 0..test_start - purge_bars - embargo_bars,
                test: test_start..total,
            }],
        })
    }
    pub fn walk_forward(
        total: usize,
        config: WalkForwardConfig,
    ) -> Result<Self, OptimizationError> {
        if config.train_bars == 0
            || config.test_bars == 0
            || config.step_bars == 0
            || config.purge_bars + config.embargo_bars >= config.train_bars
        {
            return Err(OptimizationError::InvalidFold {
                detail: "zero or exhausted window".into(),
            });
        }
        let gap = config.purge_bars + config.embargo_bars;
        let mut folds = Vec::new();
        let mut test_start = config.train_bars + gap;
        while test_start
            .checked_add(config.test_bars)
            .is_some_and(|end| end <= total)
        {
            let train_start = if config.anchored {
                0
            } else {
                test_start - gap - config.train_bars
            };
            folds.push(Fold {
                train: train_start..test_start - gap,
                test: test_start..test_start + config.test_bars,
            });
            if folds.len() >= MAX_ROBUSTNESS_STAGES {
                break;
            }
            test_start += config.step_bars;
        }
        if folds.is_empty() {
            return Err(OptimizationError::InvalidFold {
                detail: "no complete causal fold".into(),
            });
        }
        Ok(Self { folds })
    }
    pub fn folds(&self) -> &[Fold] {
        &self.folds
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleRole {
    InSample,
    OutOfSample,
    Purged,
    Embargoed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OosScheme {
    Leading {
        oos_bars: usize,
    },
    Trailing {
        oos_bars: usize,
    },
    Interleaved {
        in_sample_bars: usize,
        oos_bars: usize,
    },
    Disjoint {
        windows: Vec<Range<usize>>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OosPlan {
    roles: Vec<SampleRole>,
    ranges: BTreeMap<u8, Vec<Range<usize>>>,
}
impl OosPlan {
    pub fn new(
        total: usize,
        scheme: OosScheme,
        purge_bars: usize,
        embargo_bars: usize,
    ) -> Result<Self, OptimizationError> {
        if total == 0 || total > MAX_SEARCH_COMBINATIONS {
            return Err(invalid_fold("invalid OOS population"));
        }
        let mut windows = match scheme {
            OosScheme::Leading { oos_bars } => vec![0..oos_bars],
            OosScheme::Trailing { oos_bars } => {
                if oos_bars > total {
                    return Err(invalid_fold("trailing OOS exceeds population"));
                }
                vec![total - oos_bars..total]
            }
            OosScheme::Interleaved {
                in_sample_bars,
                oos_bars,
            } => {
                if in_sample_bars == 0 || oos_bars == 0 {
                    return Err(invalid_fold("zero interleaved span"));
                }
                let mut output = Vec::new();
                let mut start = in_sample_bars;
                while start.checked_add(oos_bars).is_some_and(|end| end <= total) {
                    output.push(start..start + oos_bars);
                    if output.len() > MAX_ROBUSTNESS_STAGES {
                        return Err(invalid_fold("too many OOS windows"));
                    }
                    start = start
                        .checked_add(in_sample_bars + oos_bars)
                        .ok_or_else(|| invalid_fold("interleaved span overflow"))?;
                }
                output
            }
            OosScheme::Disjoint { windows } => windows,
        };
        if windows.is_empty() || windows.len() > MAX_ROBUSTNESS_STAGES {
            return Err(invalid_fold("empty or excessive OOS windows"));
        }
        windows.sort_by_key(|range| (range.start, range.end));
        if windows
            .iter()
            .any(|range| range.start >= range.end || range.end > total)
            || windows.windows(2).any(|pair| pair[0].end > pair[1].start)
        {
            return Err(invalid_fold("invalid or overlapping OOS windows"));
        }
        let mut roles = vec![SampleRole::InSample; total];
        for window in &windows {
            roles[window.clone()].fill(SampleRole::OutOfSample);
        }
        for window in &windows {
            let purge = window.start.saturating_sub(purge_bars)..window.start;
            let embargo = window.end..window.end.saturating_add(embargo_bars).min(total);
            for index in purge {
                if roles[index] == SampleRole::InSample {
                    roles[index] = SampleRole::Purged;
                }
            }
            for index in embargo {
                if roles[index] == SampleRole::InSample {
                    roles[index] = SampleRole::Embargoed;
                }
            }
        }
        if !roles.contains(&SampleRole::InSample) || !roles.contains(&SampleRole::OutOfSample) {
            return Err(invalid_fold("OOS plan exhausted IS or OOS evidence"));
        }
        let mut ranges = BTreeMap::new();
        for role in [
            SampleRole::InSample,
            SampleRole::OutOfSample,
            SampleRole::Purged,
            SampleRole::Embargoed,
        ] {
            ranges.insert(role_key(role), collect_role_ranges(&roles, role));
        }
        Ok(Self { roles, ranges })
    }
    pub fn roles(&self) -> &[SampleRole] {
        &self.roles
    }
    pub fn role_ranges(&self, role: SampleRole) -> &[Range<usize>] {
        self.ranges
            .get(&role_key(role))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}
fn role_key(role: SampleRole) -> u8 {
    match role {
        SampleRole::InSample => 0,
        SampleRole::OutOfSample => 1,
        SampleRole::Purged => 2,
        SampleRole::Embargoed => 3,
    }
}
fn collect_role_ranges(roles: &[SampleRole], wanted: SampleRole) -> Vec<Range<usize>> {
    let mut output = Vec::new();
    let mut start = None;
    for (index, role) in roles.iter().enumerate() {
        if *role == wanted {
            start.get_or_insert(index);
        } else if let Some(begin) = start.take() {
            output.push(begin..index);
        }
    }
    if let Some(begin) = start {
        output.push(begin..roles.len());
    }
    output
}
fn invalid_fold(detail: &str) -> OptimizationError {
    OptimizationError::InvalidFold {
        detail: detail.into(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WalkForwardWindowEvidence {
    pub fold: Fold,
    pub selected_candidate_id: String,
    pub evaluations_n: usize,
    pub is_score: f64,
    pub oos_score: f64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct WalkForwardEvidence {
    plan: FoldPlan,
    windows: Vec<WalkForwardWindowEvidence>,
    concatenated_oos: Vec<Range<usize>>,
    degradation_bps: Vec<i32>,
}
impl WalkForwardEvidence {
    pub fn new(
        plan: FoldPlan,
        windows: Vec<WalkForwardWindowEvidence>,
    ) -> Result<Self, OptimizationError> {
        if windows.len() != plan.folds.len()
            || windows.is_empty()
            || windows.len() > MAX_ROBUSTNESS_STAGES
        {
            return Err(OptimizationError::InvalidObservation);
        }
        let mut concatenated_oos = Vec::with_capacity(windows.len());
        let mut degradation_bps = Vec::with_capacity(windows.len());
        for (expected, evidence) in plan.folds.iter().zip(&windows) {
            if &evidence.fold != expected
                || evidence.selected_candidate_id.trim().is_empty()
                || evidence.evaluations_n == 0
                || evidence.evaluations_n > MAX_TRIAL_BUDGET
                || !evidence.is_score.is_finite()
                || !evidence.oos_score.is_finite()
                || evidence.is_score <= 0.0
            {
                return Err(OptimizationError::InvalidObservation);
            }
            concatenated_oos.push(evidence.fold.test.clone());
            degradation_bps
                .push(((evidence.oos_score / evidence.is_score - 1.0) * 10_000.0).round() as i32);
        }
        Ok(Self {
            plan,
            windows,
            concatenated_oos,
            degradation_bps,
        })
    }
    pub fn concatenated_oos(&self) -> &[Range<usize>] {
        &self.concatenated_oos
    }
    pub fn degradation_bps(&self) -> &[i32] {
        &self.degradation_bps
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct WalkForwardMatrixCell {
    train_bars: usize,
    test_bars: usize,
    evidence: WalkForwardEvidence,
}
impl WalkForwardMatrixCell {
    pub fn new(
        train_bars: usize,
        test_bars: usize,
        evidence: WalkForwardEvidence,
    ) -> Result<Self, OptimizationError> {
        if train_bars == 0 || test_bars == 0 {
            return Err(OptimizationError::InvalidObservation);
        }
        Ok(Self {
            train_bars,
            test_bars,
            evidence,
        })
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct WalkForwardMatrix {
    cells: Vec<WalkForwardMatrixCell>,
}
impl WalkForwardMatrix {
    pub fn new(mut cells: Vec<WalkForwardMatrixCell>) -> Result<Self, OptimizationError> {
        if cells.is_empty() || cells.len() > MAX_ROBUSTNESS_STAGES {
            return Err(OptimizationError::InvalidObservation);
        }
        cells.sort_by_key(|cell| (cell.train_bars, cell.test_bars));
        if cells.windows(2).any(|pair| {
            (pair[0].train_bars, pair[0].test_bars) == (pair[1].train_bars, pair[1].test_bars)
        }) {
            return Err(OptimizationError::InvalidObservation);
        }
        Ok(Self { cells })
    }
    pub fn cells(&self) -> &[WalkForwardMatrixCell] {
        &self.cells
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageAccess {
    Search,
    Robustness,
    FinalReview,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataRegion {
    Search,
    FinalHoldout,
}
#[derive(Debug)]
pub struct SearchDataLease {
    stage: StageAccess,
    dataset_id: String,
    range: Range<usize>,
}
impl SearchDataLease {
    pub(crate) fn exact_partition(
        stage: StageAccess,
        dataset_id: impl Into<String>,
        range: Range<usize>,
    ) -> Result<Self, OptimizationError> {
        let dataset_id = dataset_id.into();
        if !matches!(stage, StageAccess::Search | StageAccess::Robustness)
            || dataset_id.trim().is_empty()
            || range.start >= range.end
        {
            return Err(OptimizationError::InvalidFold {
                detail: "invalid exact content partition".into(),
            });
        }
        Ok(Self {
            stage,
            dataset_id,
            range,
        })
    }
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
    pub fn stage(&self) -> StageAccess {
        self.stage
    }
    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }
}

/// Linear final-review capability. It is deliberately not `Clone` and is only created by
/// consuming the quarantine, so no API can return to search after the holdout is burned.
#[derive(Debug)]
pub struct BurnedHoldout {
    dataset_id: String,
    range: Range<usize>,
    reason: String,
}
impl BurnedHoldout {
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }
}

pub struct HoldoutQuarantine {
    search_dataset_id: String,
    final_holdout_dataset_id: String,
    total: usize,
    holdout: usize,
}
impl HoldoutQuarantine {
    pub fn new(
        search_dataset_id: impl Into<String>,
        final_holdout_dataset_id: impl Into<String>,
        total: usize,
        holdout: usize,
    ) -> Result<Self, OptimizationError> {
        let search_dataset_id = search_dataset_id.into();
        let final_holdout_dataset_id = final_holdout_dataset_id.into();
        if holdout == 0
            || holdout >= total
            || search_dataset_id.trim().is_empty()
            || final_holdout_dataset_id.trim().is_empty()
            || search_dataset_id == final_holdout_dataset_id
        {
            return Err(OptimizationError::InvalidFold {
                detail: "invalid final holdout".into(),
            });
        }
        Ok(Self {
            search_dataset_id,
            final_holdout_dataset_id,
            total,
            holdout,
        })
    }
    pub fn search_range(&self) -> Result<Range<usize>, OptimizationError> {
        Ok(0..self.total - self.holdout)
    }
    pub fn lease(&self, stage: StageAccess) -> Result<SearchDataLease, OptimizationError> {
        match stage {
            StageAccess::Search | StageAccess::Robustness => Ok(SearchDataLease {
                stage,
                dataset_id: self.search_dataset_id.clone(),
                range: self.search_range()?,
            }),
            StageAccess::FinalReview => Err(OptimizationError::HoldoutForbidden),
        }
    }
    pub fn range_for(
        &self,
        stage: StageAccess,
        region: DataRegion,
    ) -> Result<Range<usize>, OptimizationError> {
        match (stage, region) {
            (StageAccess::Search | StageAccess::Robustness, DataRegion::FinalHoldout) => {
                Err(OptimizationError::HoldoutForbidden)
            }
            (_, DataRegion::Search) => self.search_range(),
            (StageAccess::FinalReview, DataRegion::FinalHoldout) => {
                Err(OptimizationError::HoldoutForbidden)
            }
        }
    }
    pub fn burn(self, reason: impl Into<String>) -> Result<BurnedHoldout, OptimizationError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(OptimizationError::InvalidObservation);
        }
        Ok(BurnedHoldout {
            dataset_id: self.final_holdout_dataset_id,
            range: self.total - self.holdout..self.total,
            reason,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservationRole {
    SearchEvaluation,
    InSample,
    OutOfSample,
    CrossCheck,
}

/// A bounded metric projection from an already sealed strategy report. Construction verifies the
/// retest identity, report identity, run manifest and search-partition lease before exposing values.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportObservation {
    candidate_id: String,
    report_id: String,
    role: ObservationRole,
    /// The exact search partition this evidence was produced on. Retained so a consumer cannot
    /// mix evidence from two datasets — or from two different holdout splits of one dataset —
    /// into a single verdict (§7.8).
    dataset_id: String,
    stage: StageAccess,
    range: Range<usize>,
    metrics: BTreeMap<String, f64>,
}
impl ReportObservation {
    pub fn from_report(
        lease: &SearchDataLease,
        role: ObservationRole,
        request: &RetestRequest,
        result: &RetestResult,
        report: &StrategyReportArtifact,
        metric_ids: &[&str],
    ) -> Result<Self, OptimizationError> {
        if request.stage != lease.stage
            || request.dataset_id != lease.dataset_id
            || request.range_start != lease.range.start
            || request.range_end != lease.range.end
            || metric_ids.is_empty()
            || metric_ids.len() > MAX_ROBUSTNESS_STAGES
        {
            return Err(OptimizationError::InvalidObservation);
        }
        request.validate()?;
        if request.request_id != request.compute_id() {
            return Err(OptimizationError::InvalidArtifact(
                "retest request identity mismatch".into(),
            ));
        }
        result.verify_against(request)?;
        report
            .verify()
            .map_err(|error| OptimizationError::InvalidArtifact(error.to_string()))?;
        if result.report_id() != report.report_id() {
            return Err(OptimizationError::InvalidArtifact(
                "retest result does not name the supplied report".into(),
            ));
        }
        let manifest = report.run_manifest().ok_or_else(|| {
            OptimizationError::InvalidArtifact("report lacks a sealed run manifest".into())
        })?;
        let binding = manifest.binding();
        if binding.strategy_id != request.strategy_id
            || binding.config_id != request.execution_config_id
            || binding.metrics_version != request.metrics_version
            || binding.seed != request.root_seed
            || !binding
                .datasets
                .iter()
                .any(|dataset| dataset.dataset_id == request.dataset_id)
        {
            return Err(OptimizationError::InvalidArtifact(
                "report run manifest does not match retest request".into(),
            ));
        }
        let mut metrics = BTreeMap::new();
        for metric_id in metric_ids {
            if metric_id.trim().is_empty() || metrics.contains_key(*metric_id) {
                return Err(OptimizationError::InvalidObservation);
            }
            let value = match report.analysis().metric(metric_id) {
                Some(MetricValue::Defined { value }) if value.is_finite() => *value,
                _ => return Err(OptimizationError::InvalidObservation),
            };
            metrics.insert(
                (*metric_id).to_string(),
                if value == 0.0 { 0.0 } else { value },
            );
        }
        Ok(Self {
            candidate_id: request.strategy_id.clone(),
            report_id: report.report_id().to_string(),
            role,
            dataset_id: lease.dataset_id.clone(),
            stage: lease.stage,
            range: lease.range.clone(),
            metrics,
        })
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn report_id(&self) -> &str {
        &self.report_id
    }
    pub fn role(&self) -> ObservationRole {
        self.role
    }
    pub fn metric(&self, metric_id: &str) -> Option<f64> {
        self.metrics.get(metric_id).copied()
    }
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
    /// Whether this evidence was produced on exactly the partition `lease` grants.
    fn on_leased_partition(&self, lease: &SearchDataLease) -> bool {
        self.dataset_id == lease.dataset_id && self.range == lease.range
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Percentile {
    P05,
    Median,
    P95,
}

/// Nearest-rank percentile index — `ceil(bps / 10_000 * len) - 1` — over an ascending sample.
///
/// Integer-only, so one sample always resolves to one index. A `(len - 1) * bps / 10_000` form
/// truncates the upper tail into the sample minimum once `len` is small: a two-sample p95 would
/// name the *worse* observation, which inverts the confidence bound it is reported as (§7.3).
pub(crate) fn percentile_index(len: usize, bps: usize) -> usize {
    len.saturating_mul(bps).div_ceil(10_000).clamp(1, len) - 1
}
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceDistribution {
    pub role: ObservationRole,
    pub metric_id: String,
    pub observations_n: usize,
    pub sorted_samples: Vec<f64>,
    pub p05: f64,
    pub median: f64,
    pub p95: f64,
}
impl EvidenceDistribution {
    fn from_observations(
        observations: &[ReportObservation],
        role: ObservationRole,
        metric_id: &str,
    ) -> Result<Self, OptimizationError> {
        let mut sorted_samples = observations
            .iter()
            .filter(|observation| observation.role == role)
            .map(|observation| {
                observation
                    .metric(metric_id)
                    .ok_or(OptimizationError::InvalidObservation)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if sorted_samples.is_empty() || sorted_samples.len() > MAX_TRIAL_BUDGET {
            return Err(OptimizationError::InvalidObservation);
        }
        sorted_samples.sort_by(f64::total_cmp);
        let pick = |bps: usize| sorted_samples[percentile_index(sorted_samples.len(), bps)];
        Ok(Self {
            role,
            metric_id: metric_id.to_string(),
            observations_n: sorted_samples.len(),
            p05: pick(500),
            median: pick(5_000),
            p95: pick(9_500),
            sorted_samples,
        })
    }
    fn percentile(&self, percentile: Percentile) -> f64 {
        match percentile {
            Percentile::P05 => self.p05,
            Percentile::Median => self.median,
            Percentile::P95 => self.p95,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Threshold {
    AtLeast(f64),
    AtMost(f64),
}
#[derive(Debug, Clone, PartialEq)]
enum RobustnessStageKind {
    MetricPercentile {
        role: ObservationRole,
        metric_id: String,
        percentile: Percentile,
        threshold: Threshold,
    },
    DegradationRatio {
        metric_id: String,
        percentile: Percentile,
        minimum_ratio_bps: u32,
    },
}
#[derive(Debug, Clone, PartialEq)]
pub struct RobustnessStageSpec {
    order: u16,
    stage: String,
    kind: RobustnessStageKind,
}
impl RobustnessStageSpec {
    pub fn metric_percentile(
        order: u16,
        stage: impl Into<String>,
        role: ObservationRole,
        metric_id: impl Into<String>,
        percentile: Percentile,
        threshold: Threshold,
    ) -> Self {
        Self {
            order,
            stage: stage.into(),
            kind: RobustnessStageKind::MetricPercentile {
                role,
                metric_id: metric_id.into(),
                percentile,
                threshold,
            },
        }
    }
    pub fn degradation_ratio(
        order: u16,
        stage: impl Into<String>,
        metric_id: impl Into<String>,
        percentile: Percentile,
        minimum_ratio_bps: u32,
    ) -> Self {
        Self {
            order,
            stage: stage.into(),
            kind: RobustnessStageKind::DegradationRatio {
                metric_id: metric_id.into(),
                percentile,
                minimum_ratio_bps,
            },
        }
    }
    fn validate(&self) -> Result<(), OptimizationError> {
        let finite_threshold = match self.kind {
            RobustnessStageKind::MetricPercentile { threshold, .. } => match threshold {
                Threshold::AtLeast(value) | Threshold::AtMost(value) => value.is_finite(),
            },
            RobustnessStageKind::DegradationRatio {
                minimum_ratio_bps, ..
            } => minimum_ratio_bps <= 10_000,
        };
        let metric_id = match &self.kind {
            RobustnessStageKind::MetricPercentile { metric_id, .. }
            | RobustnessStageKind::DegradationRatio { metric_id, .. } => metric_id,
        };
        if self.stage.trim().is_empty() || metric_id.trim().is_empty() || !finite_threshold {
            return Err(OptimizationError::InvalidObservation);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RobustnessPipeline {
    stages: Vec<RobustnessStageSpec>,
}
impl RobustnessPipeline {
    pub fn new(mut stages: Vec<RobustnessStageSpec>) -> Result<Self, OptimizationError> {
        if stages.is_empty() || stages.len() > MAX_ROBUSTNESS_STAGES {
            return Err(OptimizationError::InvalidObservation);
        }
        for stage in &stages {
            stage.validate()?;
        }
        stages.sort_by(|left, right| (left.order, &left.stage).cmp(&(right.order, &right.stage)));
        if stages
            .windows(2)
            .any(|pair| pair[0].order == pair[1].order || pair[0].stage == pair[1].stage)
        {
            return Err(OptimizationError::InvalidObservation);
        }
        Ok(Self { stages })
    }
    pub fn execute(
        &self,
        lease: &SearchDataLease,
        candidate_id: &str,
        evaluations_n: usize,
        mut observations: Vec<ReportObservation>,
    ) -> Result<PipelineOutcome, OptimizationError> {
        if !matches!(lease.stage, StageAccess::Search | StageAccess::Robustness)
            || candidate_id.trim().is_empty()
            || evaluations_n == 0
            || evaluations_n > MAX_TRIAL_BUDGET
            || observations.is_empty()
            || observations.len() > MAX_TRIAL_BUDGET
            || observations.iter().any(|observation| {
                observation.candidate_id != candidate_id
                    || !observation.on_leased_partition(lease)
                    || observation.stage == StageAccess::FinalReview
            })
        {
            return Err(OptimizationError::InvalidObservation);
        }
        observations.sort_by(|left, right| {
            (left.role, left.report_id.as_str()).cmp(&(right.role, right.report_id.as_str()))
        });
        if observations
            .windows(2)
            .any(|pair| pair[0].report_id == pair[1].report_id)
        {
            return Err(OptimizationError::InvalidObservation);
        }
        let mut evidence = Vec::with_capacity(self.stages.len());
        let mut distributions = Vec::with_capacity(self.stages.len() * 2);
        let mut failed_stage = None;
        for spec in &self.stages {
            let (passed, reason, observations_n) = match &spec.kind {
                RobustnessStageKind::MetricPercentile {
                    role,
                    metric_id,
                    percentile,
                    threshold,
                } => {
                    let distribution =
                        EvidenceDistribution::from_observations(&observations, *role, metric_id)?;
                    let value = distribution.percentile(*percentile);
                    let passed = match threshold {
                        Threshold::AtLeast(minimum) => value >= *minimum,
                        Threshold::AtMost(maximum) => value <= *maximum,
                    };
                    let reason =
                        format!("{percentile:?} {metric_id}={value:.12}; threshold={threshold:?}");
                    let count = distribution.observations_n;
                    distributions.push(distribution);
                    (passed, reason, count)
                }
                RobustnessStageKind::DegradationRatio {
                    metric_id,
                    percentile,
                    minimum_ratio_bps,
                } => {
                    let in_sample = EvidenceDistribution::from_observations(
                        &observations,
                        ObservationRole::InSample,
                        metric_id,
                    )?;
                    let out_of_sample = EvidenceDistribution::from_observations(
                        &observations,
                        ObservationRole::OutOfSample,
                        metric_id,
                    )?;
                    let baseline = in_sample.percentile(*percentile);
                    if baseline <= 0.0 {
                        return Err(OptimizationError::InvalidObservation);
                    }
                    let ratio = (out_of_sample.percentile(*percentile) / baseline * 10_000.0)
                        .round() as i64;
                    let count = in_sample.observations_n + out_of_sample.observations_n;
                    let reason = format!(
                        "OOS/IS {percentile:?} ratio={ratio} bps; required={minimum_ratio_bps} bps"
                    );
                    distributions.push(in_sample);
                    distributions.push(out_of_sample);
                    (ratio >= i64::from(*minimum_ratio_bps), reason, count)
                }
            };
            evidence.push(if passed {
                StageEvidence::pass(&spec.stage, observations_n, reason)
            } else {
                failed_stage = Some(spec.stage.clone());
                StageEvidence::fail(&spec.stage, observations_n, reason)
            });
            if !passed {
                break;
            }
        }
        let artifact = RobustnessArtifact::seal(candidate_id, evaluations_n, evidence)?;
        Ok(PipelineOutcome {
            artifact,
            distributions,
            failed_stage,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineOutcome {
    artifact: RobustnessArtifact,
    distributions: Vec<EvidenceDistribution>,
    failed_stage: Option<String>,
}
impl PipelineOutcome {
    pub fn artifact(&self) -> &RobustnessArtifact {
        &self.artifact
    }
    pub fn distributions(&self) -> &[EvidenceDistribution] {
        &self.distributions
    }
    pub fn executed_stages(&self) -> usize {
        self.artifact.stages.len()
    }
    pub fn failed_stage(&self) -> Option<&str> {
        self.failed_stage.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveDirection {
    Maximize,
    Minimize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveSpec {
    metric_id: String,
    direction: ObjectiveDirection,
}
impl ObjectiveSpec {
    pub fn new(
        metric_id: impl Into<String>,
        direction: ObjectiveDirection,
    ) -> Result<Self, OptimizationError> {
        let metric_id = metric_id.into();
        if metric_id.trim().is_empty() {
            return Err(OptimizationError::InvalidObservation);
        }
        Ok(Self {
            metric_id,
            direction,
        })
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct BestResult {
    candidate_id: String,
    values: Vec<f64>,
    evaluations_n: usize,
}
impl BestResult {
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn label(&self) -> String {
        format!("best of N={}: {}", self.evaluations_n, self.candidate_id)
    }
}
pub fn select_best(
    observations: &[ReportObservation],
    objective: &ObjectiveSpec,
) -> Result<BestResult, OptimizationError> {
    let front = pareto_front(observations, std::slice::from_ref(objective))?;
    front
        .members
        .first()
        .cloned()
        .ok_or(OptimizationError::InvalidObservation)
}
#[derive(Debug, Clone, PartialEq)]
pub struct ParetoFront {
    members: Vec<BestResult>,
    evaluations_n: usize,
}
impl ParetoFront {
    pub fn members(&self) -> &[BestResult] {
        &self.members
    }
    pub fn label(&self) -> String {
        format!("Pareto front of N={}", self.evaluations_n)
    }
}
pub fn pareto_front(
    observations: &[ReportObservation],
    objectives: &[ObjectiveSpec],
) -> Result<ParetoFront, OptimizationError> {
    if observations.is_empty()
        || observations.len() > MAX_TRIAL_BUDGET
        || objectives.is_empty()
        || objectives.len() > MAX_ROBUSTNESS_STAGES
        || observations
            .iter()
            .any(|observation| observation.role != ObservationRole::SearchEvaluation)
    {
        return Err(OptimizationError::InvalidObservation);
    }
    let mut points = observations
        .iter()
        .map(|observation| {
            let values = objectives
                .iter()
                .map(|objective| {
                    observation
                        .metric(&objective.metric_id)
                        .ok_or(OptimizationError::InvalidObservation)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(BestResult {
                candidate_id: observation.candidate_id.clone(),
                values,
                evaluations_n: observations.len(),
            })
        })
        .collect::<Result<Vec<_>, OptimizationError>>()?;
    points.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    if points
        .windows(2)
        .any(|pair| pair[0].candidate_id == pair[1].candidate_id)
    {
        return Err(OptimizationError::InvalidObservation);
    }
    let mut members = Vec::new();
    for (index, point) in points.iter().enumerate() {
        let dominated = points.iter().enumerate().any(|(other_index, other)| {
            other_index != index && dominates(other, point, objectives)
        });
        if !dominated {
            members.push(point.clone());
        }
    }
    members.sort_by(|left, right| {
        compare_objective_values(left, right, objectives)
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });
    Ok(ParetoFront {
        members,
        evaluations_n: observations.len(),
    })
}
fn dominates(left: &BestResult, right: &BestResult, objectives: &[ObjectiveSpec]) -> bool {
    let mut strictly_better = false;
    for ((left_value, right_value), objective) in
        left.values.iter().zip(&right.values).zip(objectives)
    {
        let ordering = left_value.total_cmp(right_value);
        let no_worse = match objective.direction {
            ObjectiveDirection::Maximize => !ordering.is_lt(),
            ObjectiveDirection::Minimize => !ordering.is_gt(),
        };
        if !no_worse {
            return false;
        }
        strictly_better |= match objective.direction {
            ObjectiveDirection::Maximize => ordering.is_gt(),
            ObjectiveDirection::Minimize => ordering.is_lt(),
        };
    }
    strictly_better
}
fn compare_objective_values(
    left: &BestResult,
    right: &BestResult,
    objectives: &[ObjectiveSpec],
) -> std::cmp::Ordering {
    for ((left_value, right_value), objective) in
        left.values.iter().zip(&right.values).zip(objectives)
    {
        let ordering = match objective.direction {
            ObjectiveDirection::Maximize => right_value.total_cmp(left_value),
            ObjectiveDirection::Minimize => left_value.total_cmp(right_value),
        };
        if !ordering.is_eq() {
            return ordering;
        }
    }
    std::cmp::Ordering::Equal
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPerturbation {
    pub spread_bps: u32,
    pub slippage_bps: u32,
    pub delay_bars: u32,
}
impl ExecutionPerturbation {
    /// Produce a new sealed retest strategy whose delay is identity-bearing.
    /// Spread/slippage multipliers are carried alongside it for the caller to
    /// apply while sealing the retest execution config.
    pub fn apply_strategy(&self, strategy: &StrategyIr) -> Result<StrategyIr, OptimizationError> {
        strategy
            .verify()
            .map_err(|error| OptimizationError::InvalidArtifact(error.to_string()))?;
        let mut definition = strategy.to_input();
        definition.timing.submit_delay_bars = definition
            .timing
            .submit_delay_bars
            .checked_add(self.delay_bars)
            .filter(|delay| *delay <= crate::core::strategy_ir::MAX_SUBMIT_DELAY_BARS)
            .ok_or(OptimizationError::InvalidPerturbation)?;
        StrategyIr::build(&definition)
            .map_err(|error| OptimizationError::InvalidArtifact(error.to_string()))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPerturbationGrid {
    cases: Vec<ExecutionPerturbation>,
}
impl ExecutionPerturbationGrid {
    pub fn new(
        mut spreads: Vec<u32>,
        mut slippages: Vec<u32>,
        mut delays: Vec<u32>,
    ) -> Result<Self, OptimizationError> {
        for values in [&mut spreads, &mut slippages, &mut delays] {
            values.sort_unstable();
            values.dedup();
        }
        if spreads.is_empty()
            || slippages.is_empty()
            || delays.is_empty()
            || spreads.iter().chain(&slippages).any(|v| *v > 10_000)
            || delays.iter().any(|v| *v > 1_000)
        {
            return Err(OptimizationError::InvalidPerturbation);
        }
        let count = spreads
            .len()
            .checked_mul(slippages.len())
            .and_then(|v| v.checked_mul(delays.len()))
            .ok_or(OptimizationError::InvalidPerturbation)?;
        if count > MAX_ROBUSTNESS_STAGES {
            return Err(OptimizationError::InvalidPerturbation);
        }
        let mut cases = Vec::with_capacity(count);
        for spread_bps in spreads {
            for &slippage_bps in &slippages {
                for &delay_bars in &delays {
                    cases.push(ExecutionPerturbation {
                        spread_bps,
                        slippage_bps,
                        delay_bars,
                    });
                }
            }
        }
        Ok(Self { cases })
    }
    pub fn cases(&self) -> &[ExecutionPerturbation] {
        &self.cases
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonteCarloMethod {
    TradeOrder,
    Bootstrap,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonteCarloDistribution {
    pub method: MonteCarloMethod,
    pub seed: u64,
    pub samples: Vec<f64>,
    pub p05: f64,
    pub median: f64,
    pub p95: f64,
}
#[cfg(test)]
pub(crate) fn monte_carlo_trade_returns(
    trades: &[f64],
    method: MonteCarloMethod,
    seed: u64,
    trials: usize,
) -> Result<MonteCarloDistribution, OptimizationError> {
    if trades.is_empty()
        || trades.iter().any(|v| !v.is_finite())
        || trials == 0
        || trials > MAX_MONTE_CARLO_TRIALS
    {
        return Err(OptimizationError::InvalidObservation);
    }
    let mut rng = SplitMix64(seed);
    let mut samples = Vec::with_capacity(trials);
    let mut work = trades.to_vec();
    for _ in 0..trials {
        match method {
            MonteCarloMethod::TradeOrder => {
                work.copy_from_slice(trades);
                for i in (1..work.len()).rev() {
                    let j = (rng.next() as usize) % (i + 1);
                    work.swap(i, j);
                }
            }
            MonteCarloMethod::Bootstrap => {
                for value in &mut work {
                    *value = trades[(rng.next() as usize) % trades.len()];
                }
            }
        }
        samples.push(max_drawdown(&work));
    }
    let mut sorted = samples.clone();
    sorted.sort_by(f64::total_cmp);
    let pick = |bps: usize| sorted[percentile_index(sorted.len(), bps)];
    Ok(MonteCarloDistribution {
        method,
        seed,
        p05: pick(500),
        median: pick(5_000),
        p95: pick(9_500),
        samples,
    })
}
pub(crate) fn max_drawdown(values: &[f64]) -> f64 {
    let (mut equity, mut peak, mut drawdown) = (0.0_f64, 0.0_f64, 0.0_f64);
    for value in values {
        equity += value;
        peak = peak.max(equity);
        drawdown = drawdown.max(peak - equity);
    }
    drawdown
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariationConfig {
    pub trials: usize,
    pub trade_count: usize,
    pub trade_skip_bps: u32,
    pub parameter_jitter_bps: i32,
    pub data_noise_bps: i32,
    pub maximum_start_offset: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariationCase {
    pub kept_trade_indices: Vec<usize>,
    pub parameter_delta_bps: i32,
    pub data_noise_bps: i32,
    pub start_offset: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicVariationPlan {
    seed: u64,
    config: VariationConfig,
    cases: Vec<VariationCase>,
}
impl DeterministicVariationPlan {
    pub fn new(seed: u64, config: VariationConfig) -> Result<Self, OptimizationError> {
        if config.trials == 0
            || config.trials > MAX_ROBUSTNESS_STAGES
            || config.trade_count == 0
            || config.trade_count > MAX_TRIAL_BUDGET
            || config.trade_skip_bps > 9_999
            || !(0..=10_000).contains(&config.parameter_jitter_bps)
            || !(0..=10_000).contains(&config.data_noise_bps)
            || config.maximum_start_offset > MAX_TRIAL_BUDGET
            || config
                .trials
                .checked_mul(config.trade_count)
                .is_none_or(|count| count > MAX_SEARCH_COMBINATIONS)
        {
            return Err(OptimizationError::InvalidPerturbation);
        }
        let mut rng = SplitMix64(seed);
        let mut cases = Vec::with_capacity(config.trials);
        for _ in 0..config.trials {
            let mut kept_trade_indices = Vec::with_capacity(config.trade_count);
            for index in 0..config.trade_count {
                if rng.next() % 10_000 >= u64::from(config.trade_skip_bps) {
                    kept_trade_indices.push(index);
                }
            }
            if kept_trade_indices.is_empty() {
                kept_trade_indices.push((rng.next() as usize) % config.trade_count);
            }
            let parameter_delta_bps = symmetric_draw(&mut rng, config.parameter_jitter_bps);
            let data_noise_bps = symmetric_draw(&mut rng, config.data_noise_bps);
            let start_offset = (rng.next() as usize) % (config.maximum_start_offset + 1);
            cases.push(VariationCase {
                kept_trade_indices,
                parameter_delta_bps,
                data_noise_bps,
                start_offset,
            });
        }
        Ok(Self {
            seed,
            config,
            cases,
        })
    }
    pub fn config(&self) -> VariationConfig {
        self.config
    }
    pub fn cases(&self) -> &[VariationCase] {
        &self.cases
    }
}
fn symmetric_draw(rng: &mut SplitMix64, limit: i32) -> i32 {
    if limit == 0 {
        0
    } else {
        (rng.next() % (u64::try_from(limit).unwrap_or(0) * 2 + 1)) as i32 - limit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageVerdict {
    Pass,
    Fail,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageEvidence {
    pub stage: String,
    pub verdict: StageVerdict,
    pub observations_n: usize,
    pub reason: String,
}
impl StageEvidence {
    pub fn pass(
        stage: impl Into<String>,
        observations_n: usize,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            stage: stage.into(),
            verdict: StageVerdict::Pass,
            observations_n,
            reason: reason.into(),
        }
    }
    pub fn fail(
        stage: impl Into<String>,
        observations_n: usize,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            stage: stage.into(),
            verdict: StageVerdict::Fail,
            observations_n,
            reason: reason.into(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlateauPolicy {
    pub minimum_neighbour_ratio_bps: u32,
    pub minimum_passing_neighbours: usize,
}
pub fn parameter_plateau_evidence(
    center: f64,
    neighbours: &[f64],
    policy: PlateauPolicy,
) -> Result<StageEvidence, OptimizationError> {
    if !center.is_finite()
        || center <= 0.0
        || neighbours.is_empty()
        || neighbours.len() > MAX_ROBUSTNESS_STAGES
        || neighbours.iter().any(|v| !v.is_finite())
        || policy.minimum_neighbour_ratio_bps > 10_000
        || policy.minimum_passing_neighbours > neighbours.len()
    {
        return Err(OptimizationError::InvalidObservation);
    }
    let threshold = center * f64::from(policy.minimum_neighbour_ratio_bps) / 10_000.0;
    let passing = neighbours
        .iter()
        .filter(|value| **value >= threshold)
        .count();
    let reason = format!(
        "{passing}/{} neighbours >= {:.4} ({} bps of center)",
        neighbours.len(),
        threshold,
        policy.minimum_neighbour_ratio_bps
    );
    Ok(if passing >= policy.minimum_passing_neighbours {
        StageEvidence::pass("parameter-plateau", neighbours.len(), reason)
    } else {
        StageEvidence::fail("parameter-plateau", neighbours.len(), reason)
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParameterFieldProfile {
    pub observations_n: usize,
    pub spp_median: f64,
    pub field_minimum: f64,
    pub field_maximum: f64,
    /// Conservative lower-field / median ratio, clamped to [0, 10_000].
    pub optimization_profile_stability_bps: u32,
}
pub fn parameter_field_profile(scores: &[f64]) -> Result<ParameterFieldProfile, OptimizationError> {
    if scores.is_empty()
        || scores.len() > MAX_TRIAL_BUDGET
        || scores.iter().any(|score| !score.is_finite())
    {
        return Err(OptimizationError::InvalidObservation);
    }
    let mut sorted = scores.to_vec();
    sorted.sort_by(f64::total_cmp);
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };
    if median <= 0.0 {
        return Err(OptimizationError::InvalidObservation);
    }
    let stability = (sorted[0] / median * 10_000.0).round().clamp(0.0, 10_000.0) as u32;
    Ok(ParameterFieldProfile {
        observations_n: sorted.len(),
        spp_median: median,
        field_minimum: sorted[0],
        field_maximum: sorted[sorted.len() - 1],
        optimization_profile_stability_bps: stability,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrossCheckObservation {
    pub label: String,
    pub score: f64,
}
impl CrossCheckObservation {
    pub fn new(label: impl Into<String>, score: f64) -> Result<Self, OptimizationError> {
        let observation = Self {
            label: label.into(),
            score,
        };
        if observation.label.trim().is_empty() || !observation.score.is_finite() {
            return Err(OptimizationError::InvalidObservation);
        }
        Ok(observation)
    }
}
pub fn cross_check_gate(
    baseline: f64,
    observations: &[CrossCheckObservation],
    minimum_ratio_bps: u32,
) -> Result<StageEvidence, OptimizationError> {
    if !baseline.is_finite()
        || baseline <= 0.0
        || observations.is_empty()
        || observations.len() > MAX_ROBUSTNESS_STAGES
        || minimum_ratio_bps > 10_000
    {
        return Err(OptimizationError::InvalidObservation);
    }
    let mut labels = BTreeSet::new();
    if observations.iter().any(|observation| {
        observation.label.trim().is_empty()
            || !observation.score.is_finite()
            || !labels.insert(observation.label.as_str())
    }) {
        return Err(OptimizationError::InvalidObservation);
    }
    let minimum = observations
        .iter()
        .map(|observation| observation.score)
        .min_by(f64::total_cmp)
        .ok_or(OptimizationError::InvalidObservation)?;
    let ratio = (minimum / baseline * 10_000.0).round() as i64;
    let reason = format!("worst cross-check ratio {ratio} bps; required {minimum_ratio_bps} bps");
    Ok(if ratio >= i64::from(minimum_ratio_bps) {
        StageEvidence::pass("cross-check", observations.len(), reason)
    } else {
        StageEvidence::fail("cross-check", observations.len(), reason)
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultipleTestingAdjustment {
    pub raw_p: f64,
    pub adjusted_p: f64,
    pub evaluations_n: usize,
}
pub fn bounded_bonferroni_adjustment(
    raw_p: f64,
    evaluations_n: usize,
) -> Result<MultipleTestingAdjustment, OptimizationError> {
    if !raw_p.is_finite()
        || !(0.0..=1.0).contains(&raw_p)
        || evaluations_n == 0
        || evaluations_n > MAX_TRIAL_BUDGET
    {
        return Err(OptimizationError::InvalidObservation);
    }
    Ok(MultipleTestingAdjustment {
        raw_p,
        adjusted_p: (raw_p * evaluations_n as f64).min(1.0),
        evaluations_n,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProblemObservations {
    pub trade_count: usize,
    pub top_trade_share_bps: u32,
    pub time_in_market_bps: u32,
    pub boundary_trade_share_bps: u32,
    pub cost_2x_ratio_bps: u32,
    pub oos_is_ratio_bps: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProblemPolicy {
    pub minimum_trades: usize,
    pub maximum_top_trade_share_bps: u32,
    pub maximum_time_in_market_bps: u32,
    pub maximum_boundary_trade_share_bps: u32,
    pub minimum_cost_2x_ratio_bps: u32,
    pub minimum_oos_is_ratio_bps: u32,
}
pub fn problem_recognition_gates(
    observation: ProblemObservations,
    policy: ProblemPolicy,
) -> Result<Vec<StageEvidence>, OptimizationError> {
    let values = [
        observation.top_trade_share_bps,
        observation.time_in_market_bps,
        observation.boundary_trade_share_bps,
        observation.cost_2x_ratio_bps,
        observation.oos_is_ratio_bps,
        policy.maximum_top_trade_share_bps,
        policy.maximum_time_in_market_bps,
        policy.maximum_boundary_trade_share_bps,
        policy.minimum_cost_2x_ratio_bps,
        policy.minimum_oos_is_ratio_bps,
    ];
    if observation.trade_count == 0
        || observation.trade_count > MAX_TRIAL_BUDGET
        || policy.minimum_trades == 0
        || values.iter().any(|value| *value > 10_000)
    {
        return Err(OptimizationError::InvalidObservation);
    }
    let checks = [
        (
            "minimum-trades",
            observation.trade_count >= policy.minimum_trades,
            format!("{} >= {}", observation.trade_count, policy.minimum_trades),
        ),
        (
            "trade-concentration",
            observation.top_trade_share_bps <= policy.maximum_top_trade_share_bps,
            format!(
                "{} <= {} bps",
                observation.top_trade_share_bps, policy.maximum_top_trade_share_bps
            ),
        ),
        (
            "time-in-market",
            observation.time_in_market_bps <= policy.maximum_time_in_market_bps,
            format!(
                "{} <= {} bps",
                observation.time_in_market_bps, policy.maximum_time_in_market_bps
            ),
        ),
        (
            "boundary-reliance",
            observation.boundary_trade_share_bps <= policy.maximum_boundary_trade_share_bps,
            format!(
                "{} <= {} bps",
                observation.boundary_trade_share_bps, policy.maximum_boundary_trade_share_bps
            ),
        ),
        (
            "cost-degradation",
            observation.cost_2x_ratio_bps >= policy.minimum_cost_2x_ratio_bps,
            format!(
                "{} >= {} bps",
                observation.cost_2x_ratio_bps, policy.minimum_cost_2x_ratio_bps
            ),
        ),
        (
            "oos-degradation",
            observation.oos_is_ratio_bps >= policy.minimum_oos_is_ratio_bps,
            format!(
                "{} >= {} bps",
                observation.oos_is_ratio_bps, policy.minimum_oos_is_ratio_bps
            ),
        ),
    ];
    Ok(checks
        .into_iter()
        .map(|(stage, passed, reason)| {
            if passed {
                StageEvidence::pass(stage, observation.trade_count, reason)
            } else {
                StageEvidence::fail(stage, observation.trade_count, reason)
            }
        })
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticGatePolicy {
    pub minimum_oos_mean: f64,
    pub minimum_oos_is_ratio_bps: u32,
}
/// Literal deterministic acceptance canary. Inputs are bounded synthetic observations, not
/// backtest metrics; production results still come only from sealed reports via `RetestResult`.
pub fn synthetic_edge_gate(
    lease: &SearchDataLease,
    in_sample: &[f64],
    out_of_sample: &[f64],
    policy: SyntheticGatePolicy,
) -> Result<StageEvidence, OptimizationError> {
    if !matches!(lease.stage(), StageAccess::Search | StageAccess::Robustness)
        || lease.range.end <= lease.range.start
        || in_sample.is_empty()
        || out_of_sample.is_empty()
        || in_sample.len() + out_of_sample.len() > MAX_ROBUSTNESS_STAGES
        || in_sample
            .iter()
            .chain(out_of_sample)
            .any(|value| !value.is_finite())
        || !policy.minimum_oos_mean.is_finite()
        || policy.minimum_oos_is_ratio_bps > 10_000
    {
        return Err(OptimizationError::InvalidObservation);
    }
    let is_mean = in_sample.iter().sum::<f64>() / in_sample.len() as f64;
    let oos_mean = out_of_sample.iter().sum::<f64>() / out_of_sample.len() as f64;
    if is_mean <= 0.0 {
        return Err(OptimizationError::InvalidObservation);
    }
    let ratio_bps = (oos_mean / is_mean * 10_000.0).round() as i64;
    let passed = oos_mean >= policy.minimum_oos_mean
        && ratio_bps >= i64::from(policy.minimum_oos_is_ratio_bps);
    let reason = format!("OOS mean {oos_mean:.6}, IS mean {is_mean:.6}, ratio {ratio_bps} bps");
    Ok(if passed {
        StageEvidence::pass("synthetic-stable-edge", out_of_sample.len(), reason)
    } else {
        StageEvidence::fail("synthetic-curve-fit", out_of_sample.len(), reason)
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RobustnessArtifact {
    schema_version: u32,
    artifact_id: String,
    candidate_id: String,
    evaluations_n: usize,
    stages: Vec<StageEvidence>,
}
impl RobustnessArtifact {
    pub fn seal(
        candidate_id: impl Into<String>,
        evaluations_n: usize,
        stages: Vec<StageEvidence>,
    ) -> Result<Self, OptimizationError> {
        let mut artifact = Self {
            schema_version: 1,
            artifact_id: String::new(),
            candidate_id: candidate_id.into(),
            evaluations_n,
            stages,
        };
        artifact.validate()?;
        artifact.artifact_id = artifact.compute_id();
        Ok(artifact)
    }
    pub fn verdict(&self) -> StageVerdict {
        if self
            .stages
            .iter()
            .all(|stage| stage.verdict == StageVerdict::Pass)
        {
            StageVerdict::Pass
        } else {
            StageVerdict::Fail
        }
    }
    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn evaluations_n(&self) -> usize {
        self.evaluations_n
    }
    pub fn stages(&self) -> &[StageEvidence] {
        &self.stages
    }
    pub fn best_label(&self, score: f64) -> String {
        format!("best of N={}: {score:.6}", self.evaluations_n)
    }
    pub fn to_json_vec(&self) -> Result<Vec<u8>, OptimizationError> {
        self.verify()?;
        serde_json::to_vec(self).map_err(|e| OptimizationError::InvalidArtifact(e.to_string()))
    }
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, OptimizationError> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(OptimizationError::InvalidArtifact(
                "artifact too large".into(),
            ));
        }
        let artifact: Self = serde_json::from_slice(bytes)
            .map_err(|e| OptimizationError::InvalidArtifact(e.to_string()))?;
        artifact.verify()?;
        Ok(artifact)
    }
    pub fn verify(&self) -> Result<(), OptimizationError> {
        self.validate()?;
        if self.artifact_id != self.compute_id() {
            return Err(OptimizationError::InvalidArtifact(
                "identity mismatch".into(),
            ));
        }
        Ok(())
    }
    fn validate(&self) -> Result<(), OptimizationError> {
        if self.schema_version != 1
            || self.candidate_id.is_empty()
            || self.evaluations_n == 0
            || self.stages.is_empty()
            || self.stages.len() > MAX_ROBUSTNESS_STAGES
            || self
                .stages
                .iter()
                .any(|s| s.stage.is_empty() || s.reason.is_empty() || s.observations_n == 0)
        {
            return Err(OptimizationError::InvalidArtifact(
                "invalid structure".into(),
            ));
        }
        Ok(())
    }
    fn compute_id(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(ARTIFACT_DOMAIN);
        hasher.update(self.schema_version.to_be_bytes());
        frame(&mut hasher, self.candidate_id.as_bytes());
        hasher.update((self.evaluations_n as u64).to_be_bytes());
        hasher.update((self.stages.len() as u64).to_be_bytes());
        for stage in &self.stages {
            frame(&mut hasher, stage.stage.as_bytes());
            hasher.update([match stage.verdict {
                StageVerdict::Pass => 1,
                StageVerdict::Fail => 0,
            }]);
            hasher.update((stage.observations_n as u64).to_be_bytes());
            frame(&mut hasher, stage.reason.as_bytes());
        }
        let digest = hasher.finalize();
        let mut output = String::with_capacity(digest.len() * 2);
        for byte in digest {
            use std::fmt::Write as _;
            let _ = write!(output, "{byte:02x}");
        }
        output
    }
}
fn frame(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

#[derive(Debug)]
pub enum OptimizationJob {
    Generate {
        request_id: u64,
        space: SearchSpace,
        method: SearchMethod,
        budget: usize,
    },
}
impl OptimizationJob {
    fn request_id(&self) -> u64 {
        match self {
            Self::Generate { request_id, .. } => *request_id,
        }
    }
}
#[derive(Debug)]
pub enum OptimizationWorkerEvent {
    Completed {
        request_id: u64,
        worker_thread: std::thread::ThreadId,
        batch: SearchBatch,
    },
    Failed {
        request_id: u64,
        worker_thread: std::thread::ThreadId,
        message: String,
    },
}
#[derive(Debug)]
pub enum SubmitError {
    Backpressure(OptimizationJob),
    Disconnected(OptimizationJob),
}
pub struct OptimizationWorker {
    jobs: SyncSender<OptimizationJob>,
    events: Receiver<OptimizationWorkerEvent>,
    max_events_per_poll: usize,
}
impl OptimizationWorker {
    pub fn spawn(job_capacity: usize, event_capacity: usize) -> Result<Self, OptimizationError> {
        if job_capacity == 0 || event_capacity == 0 {
            return Err(OptimizationError::Worker(
                "queue capacity must be positive".into(),
            ));
        }
        let (job_tx, job_rx) = std::sync::mpsc::sync_channel(job_capacity);
        let (event_tx, event_rx) = std::sync::mpsc::sync_channel(event_capacity);
        std::thread::Builder::new()
            .name("strategy-optimization".into())
            .spawn(move || worker_loop(job_rx, event_tx))
            .map_err(|e| OptimizationError::Worker(e.to_string()))?;
        Ok(Self {
            jobs: job_tx,
            events: event_rx,
            max_events_per_poll: event_capacity.min(8),
        })
    }
    pub fn try_submit(&self, job: OptimizationJob) -> Result<(), SubmitError> {
        self.jobs.try_send(job).map_err(|error| match error {
            TrySendError::Full(job) => SubmitError::Backpressure(job),
            TrySendError::Disconnected(job) => SubmitError::Disconnected(job),
        })
    }
    pub fn poll(&self) -> Vec<OptimizationWorkerEvent> {
        let mut output = Vec::new();
        for _ in 0..self.max_events_per_poll {
            match self.events.try_recv() {
                Ok(event) => output.push(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        output
    }
}
fn worker_loop(jobs: Receiver<OptimizationJob>, events: SyncSender<OptimizationWorkerEvent>) {
    while let Ok(job) = jobs.recv() {
        let request_id = job.request_id();
        let worker_thread = std::thread::current().id();
        let event = match job {
            OptimizationJob::Generate {
                space,
                method,
                budget,
                ..
            } => match generate_candidates(&space, method, budget) {
                Ok(batch) => OptimizationWorkerEvent::Completed {
                    request_id,
                    worker_thread,
                    batch,
                },
                Err(error) => OptimizationWorkerEvent::Failed {
                    request_id,
                    worker_thread,
                    message: error.to_string(),
                },
            },
        };
        if events.send(event).is_err() {
            break;
        }
    }
}

pub(crate) struct SplitMix64(pub(crate) u64);
impl SplitMix64 {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests;
