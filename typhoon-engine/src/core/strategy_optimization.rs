//! Deterministic, bounded ADR-135 M4 optimizer and robustness primitives.
//!
//! This module deliberately owns orchestration and evidence, not simulation. Candidate IRs are
//! sealed through [`StrategyIr`]; callers evaluate them through the existing verified CPU run and
//! sealed-report path. The helpers here never manufacture backtest metrics or invoke the legacy
//! backtester.

use crate::core::strategy_ir::{ParamRange, ParamValue, StrategyIr};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::Mutex;
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
    HoldoutAlreadyConsumed,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BurnedHoldout {
    pub range: Range<usize>,
    pub reason: String,
}

pub struct HoldoutQuarantine {
    total: usize,
    holdout: usize,
    consumed: Mutex<bool>,
}
impl HoldoutQuarantine {
    pub fn new(total: usize, holdout: usize) -> Result<Self, OptimizationError> {
        if holdout == 0 || holdout >= total {
            return Err(OptimizationError::InvalidFold {
                detail: "invalid final holdout".into(),
            });
        }
        Ok(Self {
            total,
            holdout,
            consumed: Mutex::new(false),
        })
    }
    pub fn search_range(&self) -> Result<Range<usize>, OptimizationError> {
        if *self
            .consumed
            .lock()
            .map_err(|_| OptimizationError::Worker("holdout lock poisoned".into()))?
        {
            return Err(OptimizationError::HoldoutAlreadyConsumed);
        }
        Ok(0..self.total - self.holdout)
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
    pub fn consume_holdout(
        &self,
        reason: impl Into<String>,
    ) -> Result<BurnedHoldout, OptimizationError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(OptimizationError::InvalidObservation);
        }
        let mut consumed = self
            .consumed
            .lock()
            .map_err(|_| OptimizationError::Worker("holdout lock poisoned".into()))?;
        if *consumed {
            return Err(OptimizationError::HoldoutAlreadyConsumed);
        }
        *consumed = true;
        Ok(BurnedHoldout {
            range: self.total - self.holdout..self.total,
            reason,
        })
    }
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
pub fn monte_carlo_trade_returns(
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
    let pick = |bps: usize| sorted[(sorted.len() - 1) * bps / 10_000];
    Ok(MonteCarloDistribution {
        method,
        seed,
        p05: pick(500),
        median: pick(5_000),
        p95: pick(9_500),
        samples,
    })
}
fn max_drawdown(values: &[f64]) -> f64 {
    let (mut equity, mut peak, mut drawdown) = (0.0_f64, 0.0_f64, 0.0_f64);
    for value in values {
        equity += value;
        peak = peak.max(equity);
        drawdown = drawdown.max(peak - equity);
    }
    drawdown
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

struct SplitMix64(u64);
impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests;
