//! ADR-135 §13 M4 acceptance gate, as executable code.
//!
//! The gate sentence is literal: *a deliberately curve-fit strategy (fit to a known-random
//! synthetic series) is **rejected** by the pipeline; a synthetic strategy with a planted, genuine
//! edge **survives**; holdout access is refused by the API from within search stages; every
//! reported "best" displays its N.*
//!
//! This module executes that sentence rather than describing it. It builds a deterministic
//! synthetic corpus, publishes it through the trusted dataset store, and drives the **canonical**
//! pipeline over the store's search partition: [`execute_search_session`] (§5.5) selects a best
//! candidate from a complete plan, then [`execute_oos_scheme`] (§7.1), [`execute_cross_check_study`]
//! (§7.5), [`execute_parameter_field_study`] (§7.4) and [`execute_significance_study`] (§7.7) seal
//! the evidence [`execute_problem_recognition`] (§7.6) turns into a verdict. Nothing here computes
//! a metric, scores a run, or judges a gate: every number in an [`AcceptanceOutcome`] was produced
//! by the canonical simulator and sealed by an artifact that verifies itself.
//!
//! The two corpora differ **only in the generating process**. They share one strategy family, one
//! execution config, one search space, one policy set and one seed schedule, so the opposite
//! verdicts are a property of the data, not of a tuned threshold.
//!
//! Both processes are stated up front so "known-random" is checkable rather than asserted:
//!
//! - [`SyntheticProcess::KnownRandom`] draws every increment i.i.d. and symmetric about zero. The
//!   series is a martingale by construction, so *no* rule reading only closed bars has positive
//!   expectation on it, and a level- or oscillator-threshold rule that looks good on one partition
//!   is fitting that path's noise.
//! - [`SyntheticProcess::PlantedEdge`] adds the *same* symmetric noise to a deterministic
//!   triangular oscillation, so a mean-reversion rule has a real, repeating edge that exists on
//!   every partition and on every series the process emits — while the noise still produces losing
//!   trades and a real drawdown, which is what keeps the §7.6 "absurd metrics" gate meaningful.
//!
//! The strategy family is **scale-free** (RSI thresholds, never price levels) precisely so the
//! random walk may wander anywhere without the family losing the ability to trade it. That removes
//! the only tuning knob that could have quietly decided the outcome.

use crate::broker::alpaca::Bar;
use crate::core::strategy_builder::GeneralStrategyBuilder;
use crate::core::strategy_cross_check::{
    CrossCheckDatasetCase, CrossCheckKind, CrossCheckStudyArtifact, CrossCheckStudySpec,
    execute_cross_check_study,
};
use crate::core::strategy_dataset::{
    AdjustmentPolicy, CalendarPolicy, DatasetManifestInput, DatasetProvenance, DatasetQaPolicy,
};
use crate::core::strategy_dataset_store::{FileDatasetStore, FinalHoldoutSplit};
use crate::core::strategy_ir::{
    CompareOp, Condition, ExecutionSettings, IndicatorInput, IndicatorKind, IndicatorNode, Operand,
    ParamRange, ParamValue, SlippageModel, SpreadModel, StrategyExecutionConfig, StrategyIr,
    StrategyParameter,
};
use crate::core::strategy_optimization::{
    ObjectiveDirection, ObservationRole, OosScheme, ParameterDomain, Percentile,
    RobustnessPipeline, RobustnessStageSpec, SearchMethod, SearchSpace, SplitMix64, StageAccess,
    Threshold, generate_candidates,
};
use crate::core::strategy_parameter_field::{
    ParameterFieldStudyArtifact, ParameterFieldStudySpec, execute_parameter_field_study,
};
use crate::core::strategy_problem_recognition::{
    ProblemRecognitionArtifact, ProblemRecognitionPolicy, execute_problem_recognition,
};
use crate::core::strategy_retest::{
    ExecutedOosScheme, OosExecutionSpec, RetestError, SearchSessionArtifact, execute_oos_scheme,
    execute_search_session,
};
use crate::core::strategy_significance::{
    SignificancePolicy, SignificanceStudyArtifact, execute_significance_study,
};

/// Bars in one corpus series, one per calendar day.
///
/// Sized so the search partition alone holds thirty complete oscillations — enough closed trades
/// for §7.6's minimum-trade gate to be a real bound rather than a formality — while the whole
/// corpus stays inside one calendar year, so the coarsest calendar granularity that resolves more
/// than one period is the month rather than a single annual bucket.
pub const ACCEPTANCE_PARENT_BARS: usize = 300;
/// Bars reserved as the untouchable final holdout (§7.8). Search, robustness and every study in
/// this module see the other `ACCEPTANCE_PARENT_BARS - ACCEPTANCE_HOLDOUT_BARS`.
pub const ACCEPTANCE_HOLDOUT_BARS: usize = 60;
/// Bars in one full oscillation of [`SyntheticProcess::PlantedEdge`]: four up legs, four down.
pub const ACCEPTANCE_OSCILLATION_PERIOD: usize = 8;
/// The complete `entry_rsi x exit_rsi` grid, and therefore the selection universe N that every
/// artifact in an outcome binds and displays (§7.7).
pub const ACCEPTANCE_EVALUATIONS_N: usize = 9;

/// Half-swing of the planted oscillation, in price units around [`BASE_PRICE`].
const OSCILLATION_AMPLITUDE: f64 = 10.0;
/// Largest magnitude of one symmetric zero-mean noise draw. Shared by both processes, so the two
/// corpora carry identical noise scale and differ only in whether an oscillation sits under it.
const NOISE_AMPLITUDE: f64 = 4.0;
/// Largest magnitude of one [`SyntheticProcess::KnownRandom`] increment.
const WALK_STEP: f64 = 5.0;
const BASE_PRICE: f64 = 100.0;
/// Bars from `2026-01-01`, one per day.
const FIRST_DAY_ORDINAL: i64 = 0;

const RSI_PERIOD: f64 = 2.0;
const RSI_INDICATOR: &str = "rsi";
const ENTRY_PARAMETER: &str = "entry_rsi";
const EXIT_PARAMETER: &str = "exit_rsi";
const ENTRY_LEVELS: [f64; 3] = [25.0, 30.0, 35.0];
const EXIT_LEVELS: [f64; 3] = [65.0, 70.0, 75.0];

/// The one metric every stage of the acceptance pipeline projects, ranks and gates on.
pub const ACCEPTANCE_METRIC: &str = "net_profit";

/// The generating process behind one corpus series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticProcess {
    /// A driftless random walk: `close[i] = close[i - 1] + step`, where every `step` is an i.i.d.
    /// draw from a symmetric zero-mean set. The series is a martingale, so any rule reading only
    /// closed bars has zero expectation on it before costs and negative expectation after them.
    KnownRandom,
    /// A deterministic triangular oscillation plus the same symmetric zero-mean noise. A
    /// mean-reversion rule has a real repeating edge here; the noise still costs it trades.
    PlantedEdge,
}

impl SyntheticProcess {
    fn label(self) -> &'static str {
        match self {
            Self::KnownRandom => "known-random",
            Self::PlantedEdge => "planted-edge",
        }
    }
}

/// One synthetic series: a process, a seed, and the dataset identity it is published under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticSeriesSpec {
    pub process: SyntheticProcess,
    pub symbol: String,
    pub timeframe: String,
    pub source: String,
    pub seed: u64,
    pub bars: usize,
}

/// The complete acceptance corpus: which process to draw, from which seed, and with which
/// thresholds every §7.6 gate is judged.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptanceCorpusSpec {
    pub process: SyntheticProcess,
    /// Root of every derived stream: the series noise, the search seeds and each study's seed.
    pub seed: u64,
    pub parent_bars: usize,
    pub holdout_bars: usize,
    pub problem_policy: ProblemRecognitionPolicy,
    pub significance_policy: SignificancePolicy,
    /// Smallest cross-dataset retention (§7.5) the study will call a pass.
    pub minimum_cross_check_retention_bps: u32,
}

impl AcceptanceCorpusSpec {
    /// The corpus both M4 gate halves run: identical in every field except the process.
    pub fn gate(process: SyntheticProcess) -> Self {
        Self {
            process,
            seed: 0x4d34_ac00_1350_0001,
            parent_bars: ACCEPTANCE_PARENT_BARS,
            holdout_bars: ACCEPTANCE_HOLDOUT_BARS,
            problem_policy: gate_problem_policy(),
            significance_policy: gate_significance_policy(),
            minimum_cross_check_retention_bps: 5_000,
        }
    }
}

/// The §7.6 thresholds the gate judges **both** corpora against.
///
/// `maximum_edge_concentration_bps` is the one bound pinned at its ceiling, and deliberately: a
/// single-symbol run has exactly one symbol bucket, so its symbol family is 100 % concentrated as a
/// matter of arithmetic and no single-symbol candidate could ever clear a tighter bound. The
/// measured calendar and side shares are still derived and sealed into the artifact's observations,
/// so the evidence stays readable even where this bound cannot discriminate.
pub fn gate_problem_policy() -> ProblemRecognitionPolicy {
    ProblemRecognitionPolicy {
        minimum_trades: 15,
        maximum_top_trade_share_bps: 4_000,
        maximum_time_in_market_bps: 9_000,
        boundary_width_bps: 1_000,
        maximum_boundary_trade_share_bps: 3_000,
        minimum_cost_2x_ratio_bps: 7_000,
        minimum_cost_3x_ratio_bps: 5_000,
        minimum_oos_is_ratio_bps: 5_000,
        maximum_edge_concentration_bps: 10_000,
        maximum_absolute_sharpe_bps: 500_000,
        // Reject a literally drawdown-free report without rejecting a genuine low-drawdown edge.
        minimum_max_drawdown_bps: 1,
        minimum_parameter_step_ratio_bps: 5_000,
    }
}

/// The §7.7 thresholds the gate judges both corpora against. `null_value` is the honest null for
/// `net_profit`: a candidate is favourable at a field point only where it actually made money.
pub fn gate_significance_policy() -> SignificancePolicy {
    SignificancePolicy {
        null_value: 0.0,
        false_discovery_rate_bps: 500,
        minimum_observations: ACCEPTANCE_EVALUATIONS_N,
    }
}

/// Everything one corpus produced, in the order the pipeline sealed it.
///
/// Every field is a self-verifying artifact. `passed` is read off the sealed §7.6 verdict; this
/// type never derives one.
#[derive(Debug, Clone)]
pub struct AcceptanceOutcome {
    process: SyntheticProcess,
    search_session: SearchSessionArtifact,
    selected_strategy: StrategyIr,
    field: ParameterFieldStudyArtifact,
    significance: SignificanceStudyArtifact,
    cross_check: CrossCheckStudyArtifact,
    oos: ExecutedOosScheme,
    problem_recognition: ProblemRecognitionArtifact,
    search_dataset_id: String,
    holdout_dataset_id: String,
    search_bars: usize,
    holdout_bars: usize,
}

impl AcceptanceOutcome {
    pub fn process(&self) -> SyntheticProcess {
        self.process
    }
    /// The complete executed search the best candidate was selected from (§5.5).
    pub fn search_session(&self) -> &SearchSessionArtifact {
        &self.search_session
    }
    /// The candidate both the search session and the parameter field selected.
    pub fn selected_strategy(&self) -> &StrategyIr {
        &self.selected_strategy
    }
    pub fn field(&self) -> &ParameterFieldStudyArtifact {
        &self.field
    }
    pub fn significance(&self) -> &SignificanceStudyArtifact {
        &self.significance
    }
    pub fn cross_check(&self) -> &CrossCheckStudyArtifact {
        &self.cross_check
    }
    pub fn oos(&self) -> &ExecutedOosScheme {
        &self.oos
    }
    pub fn problem_recognition(&self) -> &ProblemRecognitionArtifact {
        &self.problem_recognition
    }
    /// The sealed §7.6 verdict: `true` only when every gate passed.
    pub fn passed(&self) -> bool {
        self.problem_recognition.passed()
    }
    /// The names of the gates that failed, in the order the engine sealed them.
    pub fn failed_gates(&self) -> Vec<&str> {
        self.problem_recognition
            .stages()
            .iter()
            .filter(|stage| stage.verdict == crate::core::strategy_optimization::StageVerdict::Fail)
            .map(|stage| stage.stage.as_str())
            .collect()
    }
    /// The selection universe the best candidate came out of (§7.7). Every artifact below binds
    /// this same count, which is what makes "best" reportable at all.
    pub fn evaluations_n(&self) -> usize {
        self.search_session.evaluations_n()
    }
    /// A "best of N" label that cannot be printed without its N.
    pub fn best_label(&self) -> String {
        format!(
            "{} best of N={}: {} on {}",
            self.process.label(),
            self.evaluations_n(),
            self.field.profile().selection_label(),
            ACCEPTANCE_METRIC
        )
    }
    pub fn search_dataset_id(&self) -> &str {
        &self.search_dataset_id
    }
    pub fn holdout_dataset_id(&self) -> &str {
        &self.holdout_dataset_id
    }
    pub fn search_bar_count(&self) -> usize {
        self.search_bars
    }
    pub fn holdout_bar_count(&self) -> usize {
        self.holdout_bars
    }
}

/// Deterministic bars for one synthetic series.
///
/// Identical inputs always produce byte-identical bars: every draw comes from a
/// [`SplitMix64`] stream derived from `spec.seed`, and the calendar is a fixed daily grid.
pub fn synthetic_bars(spec: &SyntheticSeriesSpec) -> Result<Vec<Bar>, RetestError> {
    if spec.bars < ACCEPTANCE_OSCILLATION_PERIOD * 4 || spec.bars > 100_000 {
        return Err(RetestError::Invalid(
            "synthetic series length is outside the acceptance corpus bounds".into(),
        ));
    }
    let mut rng = SplitMix64(spec.seed);
    let mut closes = Vec::with_capacity(spec.bars);
    let mut walk = BASE_PRICE;
    for index in 0..spec.bars {
        let close = match spec.process {
            SyntheticProcess::KnownRandom => {
                // Symmetric zero-mean increment: the walk is a martingale by construction.
                walk += symmetric(&mut rng, WALK_STEP);
                walk
            }
            SyntheticProcess::PlantedEdge => {
                oscillation(index) + symmetric(&mut rng, NOISE_AMPLITUDE)
            }
        };
        if !close.is_finite() || close <= 1.0 {
            return Err(RetestError::Invalid(
                "synthetic series left the priceable range".into(),
            ));
        }
        closes.push(round_tick(close));
    }
    Ok(bars_from_closes(&closes, &mut rng))
}

/// One symmetric zero-mean draw in `[-magnitude, magnitude]`, quantised to a tick so the series is
/// exactly reproducible across platforms.
fn symmetric(rng: &mut SplitMix64, magnitude: f64) -> f64 {
    // 12 bits of the stream gives 4096 evenly spaced points; the midpoint offset makes the support
    // symmetric about zero, so the draw's mean is exactly zero rather than approximately zero.
    let draw = (rng.next() >> 52) as f64;
    let unit = (draw - 2047.5) / 2047.5;
    round_tick(unit * magnitude)
}

/// The deterministic triangular wave: four legs up, four legs down, repeating forever.
fn oscillation(index: usize) -> f64 {
    let half = ACCEPTANCE_OSCILLATION_PERIOD / 2;
    let phase = index % ACCEPTANCE_OSCILLATION_PERIOD;
    let leg = if phase <= half {
        phase as f64
    } else {
        (ACCEPTANCE_OSCILLATION_PERIOD - phase) as f64
    };
    BASE_PRICE - OSCILLATION_AMPLITUDE + leg * (2.0 * OSCILLATION_AMPLITUDE / half as f64)
}

fn round_tick(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Wrap a close series into OHLCV bars on a fixed daily calendar.
///
/// Each bar opens where the previous one closed, so the series has no synthetic gaps for dataset QA
/// to flag, and its extremes are drawn from the same stream, so no bar is a carry-forward copy of
/// its predecessor.
fn bars_from_closes(closes: &[f64], rng: &mut SplitMix64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(closes.len());
    for (index, close) in closes.iter().copied().enumerate() {
        let open = if index == 0 {
            round_tick(close - 0.5)
        } else {
            closes[index - 1]
        };
        let wick = round_tick(0.05 + symmetric(rng, 0.4).abs());
        let high = round_tick(open.max(close) + wick);
        let low = round_tick((open.min(close) - wick).max(0.01));
        bars.push(Bar {
            timestamp: daily_timestamp(index),
            open,
            high,
            low,
            close,
            volume: 10_000.0 + index as f64,
        });
    }
    bars
}

/// `2026-01-01` plus `index` days, as the RFC 3339 UTC text the dataset layer stores verbatim.
///
/// One bar per calendar day, deliberately: §7.6 measures boundary reliance against the *daily*
/// calendar the report seals, so a denser intrabar grid would compress the whole run into a handful
/// of daily points and inflate that share into an artifact of the sampling rate. One bar per day
/// keeps the calendar the run's real timeline. The corpus stays inside 2026, so the coarsest
/// calendar granularity that resolves more than one period is the month.
fn daily_timestamp(index: usize) -> String {
    let date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("2026-01-01 is a valid date")
        .checked_add_signed(chrono::Duration::days(FIRST_DAY_ORDINAL + index as i64))
        .expect("the corpus calendar stays inside the representable range");
    format!("{}T00:00:00Z", date.format("%Y-%m-%d"))
}

/// The one strategy family both corpora search: a scale-free RSI mean-reversion rule whose entry
/// and exit thresholds are the only free parameters.
///
/// Scale-free is the point. A price-level rule would silently need the series to stay near the
/// levels it was written for, which would make the random walk's containment a hidden tuning knob.
/// RSI thresholds trade any series the process emits, wherever it wanders.
pub fn acceptance_strategy_family() -> Result<StrategyIr, RetestError> {
    let mut definition = GeneralStrategyBuilder::new("adr135-m4-acceptance", "typhoon")
        .definition()
        .clone();
    definition.indicators = vec![IndicatorNode {
        id: RSI_INDICATOR.into(),
        kind: IndicatorKind::Rsi,
        inputs: vec![
            IndicatorInput::Price(crate::core::strategy_ir::PriceField::Close),
            IndicatorInput::Constant(RSI_PERIOD),
        ],
    }];
    definition.parameters = vec![
        StrategyParameter {
            id: ENTRY_PARAMETER.into(),
            value: ParamValue::Float(ENTRY_LEVELS[0]),
            range: Some(ParamRange::Float {
                min: 1.0,
                max: 99.0,
            }),
        },
        StrategyParameter {
            id: EXIT_PARAMETER.into(),
            value: ParamValue::Float(EXIT_LEVELS[0]),
            range: Some(ParamRange::Float {
                min: 1.0,
                max: 99.0,
            }),
        },
    ];
    definition.long.enabled = true;
    definition.long.entry = Condition::Compare {
        left: Operand::Indicator {
            id: RSI_INDICATOR.into(),
            bars_ago: 0,
        },
        op: CompareOp::Less,
        right: Operand::Parameter(ENTRY_PARAMETER.into()),
    };
    definition.long.exit = Condition::Compare {
        left: Operand::Indicator {
            id: RSI_INDICATOR.into(),
            bars_ago: 0,
        },
        op: CompareOp::Greater,
        right: Operand::Parameter(EXIT_PARAMETER.into()),
    };
    StrategyIr::build(&definition).map_err(|error| RetestError::Invalid(error.to_string()))
}

/// The complete `entry_rsi x exit_rsi` grid. Its size is the N every artifact binds.
pub fn acceptance_search_space() -> Result<SearchSpace, RetestError> {
    let domain = |id: &str, levels: &[f64]| {
        ParameterDomain::new(id, levels.iter().copied().map(ParamValue::Float).collect())
            .map_err(|error| RetestError::Invalid(error.to_string()))
    };
    SearchSpace::new(
        acceptance_strategy_family()?,
        vec![
            domain(ENTRY_PARAMETER, &ENTRY_LEVELS)?,
            domain(EXIT_PARAMETER, &EXIT_LEVELS)?,
        ],
    )
    .map_err(|error| RetestError::Invalid(error.to_string()))
}

/// The one cost model both corpora execute under: a real spread and a real slippage distance, so
/// the §7.5 cost ladder has something to scale and a thin edge cannot survive it.
pub fn acceptance_execution_config() -> Result<StrategyExecutionConfig, RetestError> {
    let mut settings = ExecutionSettings::conservative_defaults();
    settings.spread = SpreadModel::Constant { price_units: 0.20 };
    settings.slippage = SlippageModel::FixedPriceDistance { distance: 0.25 };
    StrategyExecutionConfig::build(&settings)
        .map_err(|error| RetestError::Invalid(error.to_string()))
}

/// The terminal gate every search evaluation passes through before its report is sealed.
///
/// Selection is the search session's job, so this stage deliberately admits every finite return
/// rather than pre-filtering the field: a bound at the representable minimum records that the
/// metric was defined and leaves the ranking to §5.5.
fn acceptance_robustness_pipeline() -> Result<RobustnessPipeline, RetestError> {
    RobustnessPipeline::new(vec![RobustnessStageSpec::metric_percentile(
        1,
        "defined-search-return",
        ObservationRole::SearchEvaluation,
        ACCEPTANCE_METRIC,
        Percentile::Median,
        Threshold::AtLeast(f64::MIN),
    )])
    .map_err(|error| RetestError::Invalid(error.to_string()))
}

/// Publish one synthetic series into `store` and cut its final holdout there.
///
/// The split is minted by the store, never by this module: that is what makes the search partition
/// the *only* thing a search or robustness stage can lease (§7.8).
fn publish_split(
    store: &FileDatasetStore,
    spec: &SyntheticSeriesSpec,
    holdout_bars: usize,
) -> Result<FinalHoldoutSplit, RetestError> {
    let bars = synthetic_bars(spec)?;
    if holdout_bars == 0 || holdout_bars >= bars.len() {
        return Err(RetestError::Invalid(
            "acceptance holdout does not partition its parent".into(),
        ));
    }
    let input = DatasetManifestInput {
        symbol: spec.symbol.clone(),
        timeframe: spec.timeframe.clone(),
        provenance: DatasetProvenance {
            source: spec.source.clone(),
            venue: "synthetic".into(),
            pipeline: format!("adr135-m4-acceptance/{}/v1", spec.process.label()),
        },
        adjustment: AdjustmentPolicy::Raw,
        calendar: CalendarPolicy::Continuous24x7,
        qa_policy: DatasetQaPolicy::default(),
    };
    let record = store
        .build_and_put(&input, &bars)
        .map_err(|error| RetestError::Invalid(error.to_string()))?;
    store
        .split_final_holdout(&record.manifest.dataset_id, holdout_bars)
        .map_err(|error| RetestError::Invalid(error.to_string()))
}

/// Aggregate adjacent bar pairs into one bar of the next coarser timeframe.
///
/// This is a real timeframe transform of the same series — the §7.5 "does a 1h edge exist at 2h?"
/// question — not a relabelled copy.
fn coarser_timeframe(bars: &[Bar]) -> Vec<Bar> {
    bars.chunks_exact(2)
        .map(|pair| Bar {
            timestamp: pair[0].timestamp.clone(),
            open: pair[0].open,
            high: pair[0].high.max(pair[1].high),
            low: pair[0].low.min(pair[1].low),
            close: pair[1].close,
            volume: pair[0].volume + pair[1].volume,
        })
        .collect()
}

/// Re-quote the same series the way a second vendor would: every price moved by at most one tick,
/// deterministically, from its own stream.
///
/// §7.5's alternative-source check exists because an edge that disappears under a vendor's rounding
/// was fitting a data artifact. Applying the shift to `close` alone would leave `open` disagreeing
/// with the previous close, so the whole bar moves together.
fn alternative_source(bars: &[Bar], seed: u64) -> Vec<Bar> {
    let mut rng = SplitMix64(seed);
    let mut shifted = Vec::with_capacity(bars.len());
    let mut previous_close: Option<f64> = None;
    for bar in bars {
        let tick = round_tick(symmetric(&mut rng, 0.01));
        let close = round_tick(bar.close + tick);
        let open = previous_close.unwrap_or(round_tick(bar.open + tick));
        let high = round_tick(bar.high.max(open).max(close));
        let low = round_tick(bar.low.min(open).min(close).max(0.01));
        shifted.push(Bar {
            timestamp: bar.timestamp.clone(),
            open,
            high,
            low,
            close,
            volume: bar.volume,
        });
        previous_close = Some(close);
    }
    shifted
}

/// Execute the literal M4 acceptance pipeline for one corpus.
///
/// Every stage runs on the store's **search** partition. The final holdout is never leased, never
/// materialized and never read here; refusing it is the API's job and [`AcceptanceOutcome`] only
/// reports the id it was told to stay away from.
pub fn execute_m4_acceptance(
    store: &FileDatasetStore,
    corpus: &AcceptanceCorpusSpec,
) -> Result<AcceptanceOutcome, RetestError> {
    let config = acceptance_execution_config()?;
    let space = acceptance_search_space()?;
    let pipeline = acceptance_robustness_pipeline()?;
    let mut seeds = SplitMix64(corpus.seed);

    let baseline_seed = seeds.next();
    let split = publish_split(
        store,
        &SyntheticSeriesSpec {
            process: corpus.process,
            symbol: "SYN-A/USD".into(),
            timeframe: "1Day".into(),
            source: "synthetic-primary".into(),
            seed: baseline_seed,
            bars: corpus.parent_bars,
        },
        corpus.holdout_bars,
    )?;

    // §5.5: the complete plan is generated once and executed in full, so N is the plan's size and
    // not a count the caller chose.
    let batch = generate_candidates(&space, SearchMethod::Grid, space.combinations())
        .map_err(|error| RetestError::Invalid(error.to_string()))?;
    let evaluations_n = batch.evaluations_n;
    let search_session = execute_search_session(
        batch,
        &config,
        &split,
        &pipeline,
        ACCEPTANCE_METRIC,
        ObjectiveDirection::Maximize,
        seeds.next(),
    )?;

    // §7.4: the same complete field, executed again as a field rather than as a ranking, is what
    // SPP, the plateau and the optimization profile are derived from.
    let field = execute_parameter_field_study(
        &config,
        split.search_manifest(),
        split.search_bars(),
        split.quarantine().lease(StageAccess::Robustness)?,
        &space,
        ParameterFieldStudySpec {
            field_sample_size: evaluations_n,
            neighbour_radius: 1,
            plateau_tolerance_bps: 1_500,
            minimum_plateau_neighbours: 3,
            metric_id: ACCEPTANCE_METRIC.into(),
            direction: ObjectiveDirection::Maximize,
            root_seed: seeds.next(),
        },
    )?;
    let selected_candidate_id = field.profile().selected_candidate_id().to_string();
    if search_session.selected_strategy_id() != selected_candidate_id {
        return Err(RetestError::Invalid(
            "search session and parameter field disagree on the selected candidate".into(),
        ));
    }
    let selected_strategy = generate_candidates(&space, SearchMethod::Grid, space.combinations())
        .map_err(|error| RetestError::Invalid(error.to_string()))?
        .candidates
        .into_iter()
        .find(|candidate| candidate.candidate_id == selected_candidate_id)
        .ok_or_else(|| {
            RetestError::Invalid("the selected candidate is not a coordinate of the field".into())
        })?
        .strategy;

    let significance =
        execute_significance_study(std::slice::from_ref(&field), corpus.significance_policy)?;

    // §7.5: another series from the same process, a genuinely coarser timeframe of this one, and a
    // second vendor's quotes of this one — each published and split by the store in its own right.
    let other_symbol = publish_split(
        store,
        &SyntheticSeriesSpec {
            process: corpus.process,
            symbol: "SYN-B/USD".into(),
            timeframe: "1Day".into(),
            source: "synthetic-primary".into(),
            seed: seeds.next(),
            bars: corpus.parent_bars,
        },
        corpus.holdout_bars,
    )?;
    let adjacent_bars = coarser_timeframe(split.search_bars());
    let adjacent_manifest = crate::core::strategy_dataset::DatasetManifest::build(
        &DatasetManifestInput {
            symbol: "SYN-A/USD".into(),
            timeframe: "2Day".into(),
            provenance: DatasetProvenance {
                source: "synthetic-primary".into(),
                venue: "synthetic".into(),
                pipeline: format!(
                    "adr135-m4-acceptance/{}/resample-2/v1",
                    corpus.process.label()
                ),
            },
            adjustment: AdjustmentPolicy::Raw,
            calendar: CalendarPolicy::Continuous24x7,
            qa_policy: DatasetQaPolicy::default(),
        },
        &adjacent_bars,
    )
    .map_err(|error| RetestError::Invalid(error.to_string()))?;
    let adjacent_split = publish_bars_and_split(store, &adjacent_manifest, &adjacent_bars, 8)?;

    let alternative_bars = alternative_source(split.search_bars(), seeds.next());
    let alternative_manifest = crate::core::strategy_dataset::DatasetManifest::build(
        &DatasetManifestInput {
            symbol: "SYN-A/USD".into(),
            timeframe: "1Day".into(),
            provenance: DatasetProvenance {
                source: "synthetic-alternate".into(),
                venue: "synthetic".into(),
                pipeline: format!(
                    "adr135-m4-acceptance/{}/alternate-vendor/v1",
                    corpus.process.label()
                ),
            },
            adjustment: AdjustmentPolicy::Raw,
            calendar: CalendarPolicy::Continuous24x7,
            qa_policy: DatasetQaPolicy::default(),
        },
        &alternative_bars,
    )
    .map_err(|error| RetestError::Invalid(error.to_string()))?;
    let alternative_split =
        publish_bars_and_split(store, &alternative_manifest, &alternative_bars, 8)?;

    let cross_check = execute_cross_check_study(
        &selected_strategy,
        &config,
        split.search_manifest(),
        split.search_bars(),
        split.quarantine().lease(StageAccess::Robustness)?,
        vec![
            CrossCheckDatasetCase {
                kind: CrossCheckKind::OtherSymbol,
                label: "syn-b".into(),
                config: &config,
                dataset: other_symbol.search_manifest(),
                bars: other_symbol.search_bars(),
                lease: other_symbol.quarantine().lease(StageAccess::Robustness)?,
            },
            CrossCheckDatasetCase {
                kind: CrossCheckKind::AdjacentTimeframe,
                label: "syn-a-2day".into(),
                config: &config,
                dataset: adjacent_split.search_manifest(),
                bars: adjacent_split.search_bars(),
                lease: adjacent_split.quarantine().lease(StageAccess::Robustness)?,
            },
            CrossCheckDatasetCase {
                kind: CrossCheckKind::AlternativeSource,
                label: "syn-a-alternate".into(),
                config: &config,
                dataset: alternative_split.search_manifest(),
                bars: alternative_split.search_bars(),
                lease: alternative_split
                    .quarantine()
                    .lease(StageAccess::Robustness)?,
            },
        ],
        CrossCheckStudySpec {
            metric_id: ACCEPTANCE_METRIC.into(),
            direction: ObjectiveDirection::Maximize,
            minimum_retention_bps: corpus.minimum_cross_check_retention_bps,
            evaluations_n,
            root_seed: seeds.next(),
        },
    )?;

    // §7.1: a trailing out-of-sample region with a purge band, executed rather than inferred.
    let oos = execute_oos_scheme(
        &selected_strategy,
        &config,
        split.search_manifest(),
        split.search_bars(),
        split.quarantine().lease(StageAccess::Robustness)?,
        OosExecutionSpec {
            // Equal in-sample and out-of-sample spans, minus the purge band between them.
            // `net_profit` is extensive, so comparing a long window against a short one would
            // report a length artifact as degradation; equal windows make the ratio the strategy's.
            scheme: OosScheme::Trailing {
                oos_bars: (split.search_bars().len() - ACCEPTANCE_OSCILLATION_PERIOD) / 2,
            },
            purge_bars: ACCEPTANCE_OSCILLATION_PERIOD,
            embargo_bars: 0,
            metric_id: ACCEPTANCE_METRIC.into(),
            root_seed: seeds.next(),
        },
    )?;

    let problem_recognition =
        execute_problem_recognition(&cross_check, &oos, &significance, corpus.problem_policy)?;

    Ok(AcceptanceOutcome {
        process: corpus.process,
        search_dataset_id: split.artifact().search_dataset_id().to_string(),
        holdout_dataset_id: split.artifact().holdout_dataset_id().to_string(),
        search_bars: split.search_bars().len(),
        holdout_bars: split.artifact().range().len(),
        search_session,
        selected_strategy,
        field,
        significance,
        cross_check,
        oos,
        problem_recognition,
    })
}

/// Publish an already-built manifest's bars and cut its holdout, so every cross-check case reaches
/// the study through a store-minted search lease rather than a hand-made one.
fn publish_bars_and_split(
    store: &FileDatasetStore,
    manifest: &crate::core::strategy_dataset::DatasetManifest,
    bars: &[Bar],
    holdout_bars: usize,
) -> Result<FinalHoldoutSplit, RetestError> {
    if holdout_bars == 0 || holdout_bars >= bars.len() {
        return Err(RetestError::Invalid(
            "acceptance cross-check holdout does not partition its parent".into(),
        ));
    }
    store
        .build_and_put(&manifest.to_input(), bars)
        .map_err(|error| RetestError::Invalid(error.to_string()))?;
    store
        .split_final_holdout(&manifest.dataset_id, holdout_bars)
        .map_err(|error| RetestError::Invalid(error.to_string()))
}

#[cfg(test)]
mod tests;
