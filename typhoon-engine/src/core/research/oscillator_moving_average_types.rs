// Oscillator and moving-average research types

/// ALMA — Arnaud Legoux Moving Average with Gaussian kernel.
/// weights[i] = exp(-((i - m)^2) / (2*s^2)) where m = offset*(N-1), s = N/sigma.
/// Default length 20, offset 0.85, sigma 6.0. First Gaussian-kernel MA in the packet.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AlmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,
    pub offset: f64,
    pub sigma: f64,
    pub alma_value: f64,
    pub alma_prev: f64,
    pub deviation_pct: f64, // (last_close − alma_value) / alma_value × 100
    pub last_close: f64,
    pub alma_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// ZLEMA — Zero-Lag EMA (Ehlers). De-lag shift of `(N-1)/2` on the input
/// before computing the EMA. length 20 → lag 9.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ZlemaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,
    pub lag_shift: usize,
    pub zlema_value: f64,
    pub zlema_prev: f64,
    pub deviation_pct: f64, // (last_close − zlema_value) / zlema_value × 100
    pub last_close: f64,
    pub zlema_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// ELDERRAY — Dr. Alexander Elder's Bull/Bear Power.
/// Bull = high − EMA13(close). Bear = low − EMA13(close).
/// Dual-channel trend intensity: Bull measures upward force, Bear measures downward force.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ElderRaySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_length: usize, // 13 by default
    pub ema13: f64,
    pub ema13_prev: f64,
    pub bull_power: f64, // high − EMA13
    pub bull_power_prev: f64,
    pub bear_power: f64, // low − EMA13
    pub bear_power_prev: f64,
    pub last_close: f64,
    pub elder_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TSF — Time Series Forecast. OLS over last N closes, forecast one bar forward.
/// Projects the LINREG slope forward: forecast = slope·N + intercept (using 0..N time indices).
/// Complements LINREG (current-bar fit) with LEADING/LAGGING classification.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TsfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,       // 20
    pub slope: f64,          // per-bar slope
    pub intercept: f64,      // at t=0 of the window
    pub forecast_value: f64, // projected close at t=N (one bar ahead)
    pub last_close: f64,
    pub forecast_deviation_pct: f64, // (forecast_value − last_close) / last_close × 100
    pub r_squared: f64,              // goodness of fit
    pub tsf_label: String, // LEADING_UP / LAGGING_UP / LEADING_DOWN / LAGGING_DOWN / FLAT / INSUFFICIENT_DATA
    pub note: String,
}

/// RVI — Relative Vigor Index (John Ehlers / Markos Katsanos).
/// Numerator[i] = (C−O)[i] + 2·(C−O)[i−1] + 2·(C−O)[i−2] + (C−O)[i−3] (triangular weighting)
/// Denominator[i] = same weighting on (H−L)
/// RVI = SMA(numerator, 10) / SMA(denominator, 10)
/// Signal = (RVI[i] + 2·RVI[i−1] + 2·RVI[i−2] + RVI[i−3]) / 6
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RviSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 10
    pub rvi_value: f64,
    pub rvi_prev: f64,
    pub signal_value: f64,
    pub signal_prev: f64,
    pub last_close: f64,
    pub rvi_label: String, // BULL_CROSS / BEAR_CROSS / BULL / BEAR / NEUTRAL / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 53: TRIMA / T3 / VIDYA / SMI / PVT ───────────────────────

/// TRIMA — Triangular Moving Average. SMA-of-SMA with a (N+1)/2 sub-window
/// produces a triangular-weighted central MA. Distinct from SMA (flat),
/// WMA/HMA (linear), EMA (exponential), ALMA (Gaussian), DEMA/TEMA
/// (algebraic lag reduction). Length 20.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TrimaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub trima_value: f64,
    pub trima_prev: f64,
    pub deviation_pct: f64, // (last_close − trima_value) / trima_value × 100
    pub last_close: f64,
    pub trima_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// T3 — Tim Tillson's 1998 composite MA. Six iterative EMAs with volume
/// factor v default 0.7:
/// e1 = EMA(close, N); e2 = EMA(e1, N); ... ; e6 = EMA(e5, N);
/// c1 = −v³; c2 = 3v² + 3v³; c3 = −6v² − 3v − 3v³; c4 = 1 + 3v + v³ + 3v²;
/// T3 = c1·e6 + c2·e5 + c3·e4 + c4·e3.
/// Generalises DEMA (v=0 recovers EMA; v=1 produces strong lag reduction).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct T3Snapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub v_factor: f64, // 0.7 canonical
    pub t3_value: f64,
    pub t3_prev: f64,
    pub deviation_pct: f64,
    pub last_close: f64,
    pub t3_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// VIDYA — Tushar Chande's 1992 Variable Index Dynamic Average.
/// alpha_t = (2 / (N+1)) · |CMO(9)| / 100.
/// VIDYA_t = alpha_t · price_t + (1 − alpha_t) · VIDYA_{t−1}.
/// alpha scales with momentum: strong trends accelerate the MA, ranges freeze it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VidyaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,     // 20 (EMA base)
    pub cmo_length: usize, // 9
    pub vidya_value: f64,
    pub vidya_prev: f64,
    pub current_alpha: f64, // last-bar effective alpha
    pub cmo_magnitude: f64, // |CMO| at last bar ∈ [0, 100]
    pub deviation_pct: f64,
    pub last_close: f64,
    pub vidya_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// SMI — Stochastic Momentum Index (William Blau 1993).
/// H = max(high, N); L = min(low, N); mid = (H+L)/2.
/// Numerator = double-EMA smoothed (close − mid).
/// Denominator = double-EMA smoothed ((H−L)/2).
/// SMI = 100 · Numerator / Denominator ∈ [−100, 100].
/// Signal = EMA(SMI, short).
/// Distinct from STOCHRSI (stochastic of RSI) and STOCH (raw price stochastic).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SmiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,        // 10 lookback
    pub smooth_length: usize, // 3 double-EMA smoothing
    pub signal_length: usize, // 3 signal EMA
    pub smi_value: f64,
    pub smi_prev: f64,
    pub signal_value: f64,
    pub signal_prev: f64,
    pub last_close: f64,
    pub smi_label: String, // OVERBOUGHT / BULL_CROSS / BULL / NEUTRAL / BEAR / BEAR_CROSS / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// PVT — Price Volume Trend (Dysart/Lowry 1966).
/// PVT_t = PVT_{t−1} + volume_t · (close_t − close_{t−1}) / close_{t−1}.
/// Cumulative volume-weighted running sum of percent price changes.
/// Distinct from OBV (±volume based on close direction): PVT scales the
/// volume attribution by the magnitude of the percent move.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PvtSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pvt_value: f64, // cumulative PVT at last bar
    pub pvt_prev: f64,  // cumulative PVT at previous bar
    pub pvt_ema: f64,   // 20-bar EMA of PVT series
    pub pvt_slope: f64, // PVT[last] − PVT[last−n], n=20
    pub last_close: f64,
    pub pvt_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 54: AC / CHVOL / BBWIDTH / ELDERIMP / RMI ───────────────

/// Bill Williams's Accelerator Oscillator — a second-derivative momentum
/// indicator built as `AC = AO − SMA₅(AO)` where
/// `AO = SMA₅(medprice) − SMA₃₄(medprice)`. Flags acceleration direction
/// relative to the AO trend.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AcSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ac_value: f64, // AO − SMA5(AO)
    pub ac_prev: f64,
    pub ao_value: f64, // current Awesome Oscillator
    pub ao_sma5: f64,  // 5-SMA of AO
    pub last_close: f64,
    pub ac_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Marc Chaikin's Volatility indicator — the rate-of-change of the 10-bar
/// EMA of the high-low range over a 10-bar lookback. Positive readings
/// indicate range expansion; negative readings indicate contraction.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ChvolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_length: usize, // 10
    pub roc_length: usize, // 10
    pub chvol_value: f64,  // 100 · (EMA − EMA[−roc])/EMA[−roc]
    pub chvol_prev: f64,
    pub ema_range: f64, // EMA₁₀(H−L) at last bar
    pub last_close: f64,
    pub chvol_label: String, // EXPANDING / CONTRACTING / NEUTRAL / INSUFFICIENT_DATA
    pub note: String,
}

/// John Bollinger's Bandwidth — `BBW = (upper − lower)/middle` using the
/// standard SMA₂₀ ± 2σ bands. Low readings indicate a "squeeze" of
/// pending volatility expansion; the percentile over a 125-bar window
/// quantifies how extreme the current reading is.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BbwidthSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,  // 20
    pub num_stdev: f64, // 2.0
    pub bbw_value: f64,
    pub bbw_prev: f64,
    pub bbw_percentile: f64, // rank of bbw_value over last 125 bars, 0..100
    pub middle: f64,         // SMA₂₀
    pub upper: f64,
    pub lower: f64,
    pub last_close: f64,
    pub bbw_label: String, // SQUEEZE / LOW / NORMAL / EXPANDED / INSUFFICIENT_DATA
    pub note: String,
}

/// Alexander Elder's Impulse System — colour-codes bars using the sign
/// agreement between a 13-EMA slope and the MACD histogram. GREEN when
/// both rise, RED when both fall, BLUE (neutral/transition) otherwise.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ElderImpulseSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_length: usize, // 13
    pub ema_value: f64,
    pub ema_slope: f64, // EMA − EMA[−1]
    pub macd_hist: f64, // current MACD histogram
    pub macd_hist_prev: f64,
    pub macd_hist_slope: f64, // hist − hist[−1]
    pub last_close: f64,
    pub impulse_label: String, // GREEN / BLUE / RED / INSUFFICIENT_DATA
    pub note: String,
}

/// Roger Altman's Relative Momentum Index — RSI variant applied to the
/// N-bar momentum series `close − close[−N]` instead of the 1-bar change.
/// Tends to lag RSI slightly but produces smoother overbought/oversold
/// signals during strong trends.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RmiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,          // 14
    pub momentum_length: usize, // 5
    pub rmi_value: f64,         // 0..100
    pub rmi_prev: f64,
    pub last_close: f64,
    pub rmi_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// Wilder's Smoothed Moving Average (SMMA / RMA) — a recursive MA with
/// `SMMA_t = (SMMA_{t-1}·(N-1) + price_t) / N`. Equivalent to EMA with
/// `alpha = 1/N` (vs classical EMA's `alpha = 2/(N+1)`), giving it much
/// slower decay and less whipsaw than same-length EMA. Basis of ATR,
/// RSI's average gain/loss, and Williams's Alligator surface.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SmmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub smma_value: f64,
    pub smma_prev: f64,
    pub deviation_pct: f64, // (close − smma)/smma · 100
    pub last_close: f64,
    pub smma_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Bill Williams's Alligator — three displaced SMMAs of the median price:
/// `jaw = SMMA₁₃(medprice) shifted +8`, `teeth = SMMA₈ shifted +5`,
/// `lips = SMMA₅ shifted +3`. The current-bar values surfaced here are
/// the *shifted-to-today* values, i.e. `jaw[t] = SMMA₁₃(medprice[0..=t-8])`,
/// etc. Label encodes the alligator's state: SLEEPING when the three
/// lines are intertwined; EATING_UP when lips > teeth > jaw and spreading;
/// EATING_DOWN when lips < teeth < jaw and spreading; AWAKENING when
/// crossing. Classic chart-pattern surface in forex/crypto systems.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AlligatorSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub jaw: f64,   // SMMA₁₃, shifted +8
    pub teeth: f64, // SMMA₈,  shifted +5
    pub lips: f64,  // SMMA₅,  shifted +3
    pub jaw_prev: f64,
    pub teeth_prev: f64,
    pub lips_prev: f64,
    pub spread_pct: f64, // (max − min of 3 lines) / last_close · 100
    pub last_close: f64,
    pub alligator_label: String, // EATING_UP / EATING_DOWN / AWAKENING / SLEEPING / INSUFFICIENT_DATA
    pub note: String,
}

/// Larry Connors's Connors RSI — a composite of three momentum
/// components: `CRSI = (RSI₃(close) + RSI₂(streak) + percent_rank(ROC₁, 100))/3`.
/// `streak` is the current up/down streak counter. The percent_rank
/// component measures where today's 1-bar ROC ranks over the last 100
/// bars. Behaves as a mean-reversion signal with sharp extremes — the
/// canonical Connors entry/exit threshold is >90 (short) / <10 (long).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CrsiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rsi_length: usize,    // 3
    pub streak_length: usize, // 2
    pub rank_lookback: usize, // 100
    pub rsi_close: f64,       // RSI₃(close)
    pub rsi_streak: f64,      // RSI₂(streak)
    pub percent_rank: f64,    // 100·rank/lookback
    pub crsi_value: f64,      // mean of the three components
    pub crsi_prev: f64,
    pub last_close: f64,
    pub crsi_label: String, // OVERBOUGHT / BULLISH / NEUTRAL / BEARISH / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// Standard Error Bands — Tim Tillson / Don Fishback channels around a
/// linear regression endpoint fit. Center is the linreg fitted value at
/// `t = N−1`, bands are `center ± k·SE` where
/// `SE = sqrt(Σ(y_i − ŷ_i)² / (N−2))`. Narrower than Bollinger when the
/// fit is good (low residual variance) and wider when price is noisy
/// around the trend. Better mean-reversion signal in trending markets
/// than Bollinger since the center itself captures the trend.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SebSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub num_se: f64,   // 2.0
    pub upper: f64,
    pub middle: f64, // linreg endpoint fit
    pub lower: f64,
    pub bandwidth: f64,    // (upper − lower)/middle
    pub position_pct: f64, // (close − lower)/(upper − lower) · 100
    pub last_close: f64,
    pub seb_label: String, // ABOVE_BAND / UPPER_HALF / NEUTRAL / LOWER_HALF / BELOW_BAND / INSUFFICIENT_DATA
    pub note: String,
}

/// Tushar Chande's Intraday Momentum Index — RSI applied to the
/// **bar-by-bar close-minus-open series** rather than close-minus-prior-
/// close. `IMI = 100·Σ(up_cls-op) / (Σ(up) + Σ|down|)` over N bars.
/// Measures buying vs selling pressure within each bar, making it
/// sensitive to intraday direction. Distinct from RSI (inter-bar), QSTICK
/// (EMA of close-open), and BOP (single-bar scaled close-open).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ImiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub sum_gains: f64,
    pub sum_losses: f64,
    pub imi_value: f64, // 0..100
    pub imi_prev: f64,
    pub last_close: f64,
    pub imi_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// Daryl Guppy's Multiple Moving Average — a fan of twelve EMAs split into
/// a **short-term trader group** (3, 5, 8, 10, 12, 15) and a **long-term
/// investor group** (30, 35, 40, 45, 50, 60). When the short group is
/// above and spread wide and the long group is below and parallel, a
/// strong uptrend is confirmed. Compression in both groups signals an
/// imminent move. `compression_pct` measures the short-group width
/// relative to last close.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GmmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub short_ema_avg: f64, // mean of 3,5,8,10,12,15
    pub long_ema_avg: f64,  // mean of 30,35,40,45,50,60
    pub short_min: f64,     // min of short group
    pub short_max: f64,     // max of short group
    pub long_min: f64,
    pub long_max: f64,
    pub short_compression_pct: f64, // (short_max − short_min)/last_close · 100
    pub long_compression_pct: f64,
    pub group_gap_pct: f64, // (short_ema_avg − long_ema_avg)/last_close · 100
    pub last_close: f64,
    pub gmma_label: String, // STRONG_UPTREND / UPTREND / COMPRESSION / DOWNTREND / STRONG_DOWNTREND / INSUFFICIENT_DATA
    pub note: String,
}

/// Moving Average Envelope — a simple MA bracketed by **fixed percentage
/// bands** above and below, as distinct from Bollinger (stdev-based) or
/// Keltner (ATR-based). Classical technician's channel: `upper = MA·(1+k)`,
/// `lower = MA·(1−k)`. Position within the envelope is a coarse
/// overbought/oversold gauge.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MaenvSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub pct_band: f64, // 2.5
    pub upper: f64,
    pub middle: f64, // SMA(N)
    pub lower: f64,
    pub bandwidth_pct: f64, // 2 · pct_band (constant, for symmetry)
    pub position_pct: f64,  // (close − lower)/(upper − lower) · 100
    pub last_close: f64,
    pub maenv_label: String, // ABOVE_BAND / UPPER_HALF / NEUTRAL / LOWER_HALF / BELOW_BAND / INSUFFICIENT_DATA
    pub note: String,
}

/// Marc Chaikin's Accumulation/Distribution Line — a cumulative running
/// total of `money flow multiplier × volume`, where
/// `MFM = ((close − low) − (high − close)) / (high − low)`. Tracks whether
/// the bar closes in the upper (accumulation) or lower (distribution) half
/// of its range and weights by volume. Rising ADL with flat/down price is
/// a bullish divergence; falling ADL with flat/up price is bearish.
/// Distinct from OBV (raw signed volume) and CMF (ranged-MFM / ranged-vol
/// ratio): ADL is the cumulative running total of MFM·V.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdlSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub adl_value: f64, // cumulative total
    pub adl_prev: f64,
    pub adl_sma_length: usize, // 20
    pub adl_sma: f64,          // SMA(ADL, 20)
    pub slope_per_bar: f64,    // OLS slope of last 20 ADL points
    pub last_close: f64,
    pub price_delta_pct: f64, // close over last 20 bars vs adl direction
    pub adl_label: String, // STRONG_ACCUMULATION / ACCUMULATION / NEUTRAL / DISTRIBUTION / STRONG_DISTRIBUTION / INSUFFICIENT_DATA
    pub note: String,
}

/// Adam White's Vertical Horizontal Filter — measures **trendiness vs
/// ranging** of the price series over N bars:
/// `VHF = (HHV_N − LLV_N) / Σ|Δclose|`. High VHF (>0.5) means price is
/// grinding in one direction (trending); low VHF (<0.3) means price is
/// chopping around the same range (ranging). Distinct from ADX (which is
/// a trend strength oscillator on +DI/-DI differences), CHOP (log10 of
/// range/sum-of-TR), and AROON (positional HHV/LLV timing).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VhfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 28
    pub highest_high: f64,
    pub lowest_low: f64,
    pub sum_abs_delta: f64,
    pub vhf_value: f64,
    pub vhf_prev: f64,
    pub last_close: f64,
    pub vhf_label: String, // STRONG_TREND / TREND / NEUTRAL / RANGING / STRONG_RANGING / INSUFFICIENT_DATA
    pub note: String,
}

/// Volume Rate of Change — `VROC = (V_now − V_{now-N}) / V_{now-N} · 100`.
/// Analogous to price ROC but on the volume series. Spikes in VROC mark
/// unusual participation (news, earnings, breakouts); persistent positive
/// VROC with rising price confirms trend. Different from RelVol (which
/// compares current vs long-horizon average) and NVol (current vs 20-day
/// median); VROC is strictly a two-point volume delta.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VrocSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub volume_now: f64,
    pub volume_then: f64,
    pub vroc_value: f64, // pct
    pub vroc_prev: f64,
    pub last_close: f64,
    pub vroc_label: String, // SURGE / ELEVATED / NEUTRAL / QUIET / COLLAPSE / INSUFFICIENT_DATA
    pub note: String,
}

/// KDJ — the Chinese-market Stochastic Oscillator variant built from
/// `RSV_N = 100·(close−LLV_N)/(HHV_N−LLV_N)` over N=9 bars, then
/// `K = EMA_{1/3}(RSV)`, `D = EMA_{1/3}(K)`, and the distinguishing
/// `J = 3·K − 2·D`. J amplifies cross-overs earlier than plain %K/%D,
/// and its extreme prints (J>100 or J<0) are interpreted as aggressive
/// overbought/oversold flags. Distinct from STOCH (, bare %K/%D
/// only), STOCHRSI (, stochastic-of-RSI rather than
/// stochastic-of-price), KST (Pring's Know-Sure-Thing, multi-ROC
/// rate-of-change composite), and WILLR (, inverse %R). KDJ
/// is the one momentum surface that explicitly exposes the amplified
/// J line as a separate field rather than a derived calculation.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KdjSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub stoch_length: usize, // 9 — RSV window
    pub k_smooth: usize,     // 3 — EMA smoothing constant for K and D (α = 1/3 ⇒ period = 5)
    pub rsv: f64,            // raw stochastic %
    pub k_value: f64,        // EMA_{1/3}(RSV)
    pub d_value: f64,        // EMA_{1/3}(K)
    pub j_value: f64,        // 3·K − 2·D
    pub j_prev: f64,
    pub last_close: f64,
    pub kdj_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// QQE — Quantitative Qualitative Estimation, a smoothed RSI-based
/// trend system built by Igor Livshin. Applies 5-bar EMA smoothing to
/// the RSI (default RSI₁₄) to produce `rsi_smoothed`, then computes an
/// adaptive trailing band based on Wilder's MA of ΔRSI: `fast_atr_rsi =
/// Wilder(|ΔRSI|, 14)`, `qqe_fast = rsi_smoothed − 4.236 · fast_atr_rsi_avg`
/// for the lower band, symmetric upper. The trend line is the lagged
/// crossover of these bands. Used as both an early-trend filter and an
/// overbought/oversold gauge — crosses above 50 with trend line below
/// RSI = bullish entry.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct QqeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rsi_length: usize,     // 14
    pub smooth_length: usize,  // 5
    pub qqe_factor: f64,       // 4.236
    pub rsi_value: f64,        // raw RSI
    pub rsi_smoothed: f64,     // EMA(RSI, 5)
    pub fast_atr_rsi_avg: f64, // smoothed Wilder MA of |ΔRSI_smoothed|
    pub upper_band: f64,       // rsi_smoothed + qqe_factor · fast_atr_rsi_avg
    pub lower_band: f64,       // rsi_smoothed − qqe_factor · fast_atr_rsi_avg
    pub qqe_prev: f64,         // prior bar rsi_smoothed
    pub last_close: f64,
    pub qqe_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Martin Pring's Price Momentum Oscillator — a smoothed ROC indicator:
/// `PMO = EMA(EMA(ROC(close,1)·10, 35), 20)` followed by a 10-bar EMA
/// signal line. The heavy triple-smoothing produces a highly reactive
/// but noise-filtered momentum line. Distinct from MACD (EMA₁₂ − EMA₂₆
/// of close), from TRIX (triple-smoothed EMA of close, ), and
/// from PPO (percentage price oscillator); PMO is smoothed-ROC with a
/// signal line, designed for multi-month swing trading.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PmoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub smooth1_length: usize, // 35
    pub smooth2_length: usize, // 20
    pub signal_length: usize,  // 10
    pub pmo_value: f64,
    pub pmo_signal: f64,
    pub pmo_prev: f64,
    pub histogram: f64, // pmo − pmo_signal
    pub last_close: f64,
    pub pmo_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Tushar Chande's Forecast Oscillator — compares the current close to
/// the one-bar-ahead forecast from a linear regression fit over N bars:
/// `CFO = 100 · (close − forecast) / close`. Positive means price is
/// ahead of trend (bullish deviation), negative means behind (bearish
/// deviation). Zero crossings are trend-reversal signals in Chande's
/// systems. Distinct from LINREG (fitted value, ), TSF
/// (projected future value, ), and from PPO / DPO
/// (non-regression momentum). CFO is the one oscillator built as
/// close-minus-regression-forecast as a percentage.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CfoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,  // 14
    pub slope: f64,     // OLS slope
    pub intercept: f64, // OLS intercept
    pub forecast: f64,  // slope·N + intercept (one-bar-ahead)
    pub cfo_value: f64, // 100·(close − forecast)/close
    pub cfo_prev: f64,
    pub last_close: f64,
    pub cfo_label: String, // STRONG_ABOVE_TREND / ABOVE_TREND / NEUTRAL / BELOW_TREND / STRONG_BELOW_TREND / INSUFFICIENT_DATA
    pub note: String,
}

/// Colin Twiggs's Twiggs Money Flow — a smoothed, volume-weighted
/// variant of Chaikin Money Flow. Replaces the bar's full
/// high/low range with a *true range* (max(high, prev_close) −
/// min(low, prev_close)) to correctly handle gap bars, then smooths
/// with an exponential MA rather than a simple sum: TMF tracks
/// cumulative net volume more smoothly than raw CMF and is less
/// jittery on gap-heavy instruments. Twiggs's own default is 21-bar
/// EMA smoothing on both numerator (money flow) and denominator
/// (volume). Distinct from CMF (range-based, simple sum), ADL
/// (cumulative total, not ratio), KLINGER (dual-EMA volume force),
/// and PVT (ROC·volume).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TmfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,       // 21
    pub ema_money_flow: f64, // EMA of money_flow_volume
    pub ema_volume: f64,     // EMA of volume
    pub tmf_value: f64,      // ema_money_flow / ema_volume
    pub tmf_prev: f64,
    pub last_close: f64,
    pub tmf_label: String, // STRONG_INFLOW / INFLOW / NEUTRAL / OUTFLOW / STRONG_OUTFLOW / INSUFFICIENT_DATA
    pub note: String,
}

/// Bill Williams Fractals — 5-bar local-extremum markers. A bullish
/// (up) fractal forms when a bar's high is strictly greater than both
/// the two preceding bars' highs AND the two following bars' highs; a
/// bearish (down) fractal is the symmetric construction on lows. Used
/// as structural S/R pivots and as the building block for Williams's
/// Alligator-based entry/exit rules. Distinct from ZigZag (pct-move
/// threshold) and Pivot Points (floor-trader formula over prior OHLC).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FractalsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub window: usize,             // 5 — 2 left, pivot, 2 right
    pub last_up_high: f64,         // most recent bullish fractal high
    pub last_up_bars_ago: usize,   // bars since last up fractal confirmed
    pub last_down_low: f64,        // most recent bearish fractal low
    pub last_down_bars_ago: usize, // bars since last down fractal confirmed
    pub up_fractal_count: usize,   // total up fractals in scanned window
    pub down_fractal_count: usize, // total down fractals in scanned window
    pub last_close: f64,
    pub fractals_label: String, // UP_RECENT / DOWN_RECENT / BOTH_RECENT / NONE_RECENT / INSUFFICIENT_DATA
    pub note: String,
}

/// Ehlers Inverse Fisher Transform of RSI — rescales RSI to
/// [-5, 5] via `v = 0.1·(RSI − 50)`, smooths with a 9-bar WMA, then
/// applies `ift = (e^{2v} − 1) / (e^{2v} + 1)` to produce a bounded
/// [-1, 1] oscillator. The inverse Fisher transform compresses
/// mid-range values toward zero and expands extremes toward ±1,
/// sharpening reversal signals relative to raw RSI. Crossings of
/// ±0.5 are strong trend-change alerts. Distinct from raw RSI, from
/// STOCHRSI (stochastic of RSI), from QQE (smoothed RSI with
/// adaptive bands, ), and from CRSI (Connors composite,
/// ).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IftRsiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rsi_length: usize, // 14
    pub wma_length: usize, // 9
    pub rsi_value: f64,    // raw RSI₁₄
    pub v_value: f64,      // WMA₉ of 0.1·(RSI − 50)
    pub ift_value: f64,    // (e^{2v} − 1)/(e^{2v} + 1) ∈ [-1, 1]
    pub ift_prev: f64,
    pub last_close: f64,
    pub ift_rsi_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// MESA Adaptive Moving Average (Ehlers) — a phase-adaptive MA that
/// estimates the dominant cycle period via a simplified Hilbert
/// transform (in-phase and quadrature components) and then sets α
/// adaptively: `α = fast_limit / (period / 2)`, clamped to
/// `[slow_limit, fast_limit]`. The companion FAMA (Following
/// Adaptive MA) is MAMA smoothed with half its α. The fastlimit /
/// slowlimit defaults are 0.5 / 0.05. Distinct from KAMA (Kaufman,
/// efficiency-ratio-based adaptive), from T3 (Tillson triple-DEMA),
/// and from every fixed-α EMA on the shipped MA list.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MamaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_limit: f64, // 0.5
    pub slow_limit: f64, // 0.05
    pub mama_value: f64,
    pub fama_value: f64,
    pub mama_prev: f64,
    pub fama_prev: f64,
    pub alpha: f64,  // current adaptive α
    pub period: f64, // detected dominant cycle period
    pub last_close: f64,
    pub mama_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Ehlers Center of Gravity oscillator — a zero-lag oscillator built
/// as the negative weighted centroid of the last N closes:
/// `COG = -Σ_{i=0..N-1}((i+1)·close_{N-1-i}) / Σ_{i=0..N-1}(close_{N-1-i})`
/// with canonical N=10. Signal line is a 3-bar lagged copy. Ehlers
/// argued that the sign flip and the weighting by recency produce an
/// oscillator that leads traditional momentum by one bar on average.
/// Distinct from every EMA-based oscillator (MACD, TRIX, PMO), from
/// LINREG-based (LINREG/CFO), and from simple ROC.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CogSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 10
    pub cog_value: f64,
    pub cog_signal: f64, // 3-bar lag
    pub cog_prev: f64,
    pub last_close: f64,
    pub cog_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Didi Aguiar's Didi Index — a Brazilian 3-SMA crossover system
/// where three SMAs (short 3, medium 8, long 20) are normalized by
/// dividing by the medium: `short_ratio = short_sma/medium_sma − 1`,
/// `long_ratio = long_sma/medium_sma − 1`. The characteristic "didi
/// needles" pattern fires when short and long cross the zero line
/// from opposite sides — BULL_NEEDLES when short crosses up through
/// zero while long crosses down through zero, and symmetric
/// BEAR_NEEDLES. Between needle events, the ordering of short,
/// medium, and long drives the trend classification. Distinct from
/// every 2-line MA crossover (golden/death cross), from Guppy
/// (GMMA, 12-line fan, ), and from ALLIGATOR (3-line SMMA).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DidiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub short_length: usize,  // 3
    pub medium_length: usize, // 8
    pub long_length: usize,   // 20
    pub short_ratio: f64,     // short/medium − 1
    pub long_ratio: f64,      // long/medium − 1
    pub short_prev: f64,
    pub long_prev: f64,
    pub last_close: f64,
    pub didi_label: String, // BULL_NEEDLES / BULL / NEUTRAL / BEAR / BEAR_NEEDLES / INSUFFICIENT_DATA
    pub note: String,
}

/// Tom DeMark's DeMarker (DeM) — a bounded [0,1] momentum oscillator.
/// Over an N=14 lookback, DeMax[i] = max(high[i]−high[i−1], 0) and
/// DeMin[i] = max(low[i−1]−low[i], 0); summing these and taking
/// `DeM = ΣDeMax / (ΣDeMax + ΣDeMin)` produces an oscillator that
/// weights recent highs vs recent lows, so sustained up-legs push
/// DeM toward 1 and sustained down-legs push it toward 0. Readings
/// above 0.7 flag overbought conditions, below 0.3 oversold.
/// Distinct from RSI (Wilder smoothing of gains/losses on close),
/// from Williams %R (range-position of close), and from STOCHRSI.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DemarkerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub demax_sum: f64,
    pub demin_sum: f64,
    pub demarker_value: f64, // [0, 1]
    pub demarker_prev: f64,
    pub last_close: f64,
    pub demarker_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// Bill Williams Gator Oscillator — a companion to the Alligator
/// that visualizes how the three shifted SMMAs diverge or
/// converge. `upper = |jaws − teeth|` plotted above zero and
/// `lower = −|teeth − lips|` plotted below zero, where jaws =
/// SMMA₁₃ shifted 8 bars, teeth = SMMA₈ shifted 5, lips = SMMA₅
/// shifted 3. The Gator has four life phases: SLEEPING (both bars
/// small), AWAKENING (bars change color — one growing, one
/// shrinking), EATING (both bars growing — trend feeding), and
/// SATED (both bars shrinking — trend exhausting). Distinct from
/// ALLIGATOR (the raw MA triplet) and from every MA-spread oscillator.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GatorSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub jaw_length: usize,   // 13
    pub teeth_length: usize, // 8
    pub lips_length: usize,  // 5
    pub upper_bar: f64,      // |jaws − teeth|
    pub lower_bar: f64,      // −|teeth − lips|
    pub upper_prev: f64,
    pub lower_prev: f64,
    pub last_close: f64,
    pub gator_label: String, // SLEEPING / AWAKENING / EATING / SATED / INSUFFICIENT_DATA
    pub note: String,
}

/// Bill Williams Market Facilitation Index (BW MFI) — measures how
/// much price moved per unit of volume: `mfi = (high − low) / volume`
/// (tick-scaled). Williams then classifies each bar by comparing
/// current MFI and volume to the prior bar's values, producing four
/// colored dots: GREEN (MFI up, volume up — genuine strong move),
/// FADE (MFI down, volume down — interest fading), FAKE (MFI up,
/// volume down — false breakout) and SQUAT (MFI down, volume up —
/// indecision battle, often precedes reversal). Distinct from
/// Chaikin's MFI (, based on money-flow volume).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BwMfiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mfi_value: f64, // (high − low) / volume (tick-scaled)
    pub mfi_prev: f64,
    pub volume: f64,
    pub volume_prev: f64,
    pub last_close: f64,
    pub bwmfi_color: String, // GREEN / FADE / FAKE / SQUAT
    pub bwmfi_label: String, // GREEN / FADE / FAKE / SQUAT / INSUFFICIENT_DATA
    pub note: String,
}

/// Volume Weighted Moving Average (VWMA) — a simple moving average
/// of close weighted by volume: `vwma = Σ(close·vol) / Σ(vol)` over
/// N=20. High-volume closes dominate the average, so VWMA diverges
/// from the plain SMA when recent volume spikes above the baseline,
/// providing an institutional-footprint smoother. The VWMA−SMA
/// spread is the core signal: positive when big volume aligns with
/// higher prices, negative when big volume aligns with lower prices.
/// Distinct from VWAP (session-anchored, ) and from every
/// other fixed-length MA (SMA, EMA, HMA, DEMA, ALMA, KAMA, MAMA).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VwmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub vwma_value: f64,
    pub sma_value: f64,
    pub vwma_prev: f64,
    pub spread: f64,       // vwma − sma
    pub spread_ratio: f64, // (vwma − sma) / sma
    pub last_close: f64,
    pub vwma_label: String, // BULL / WEAK_BULL / NEUTRAL / WEAK_BEAR / BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Rolling sample standard deviation of close over N=20 with a
/// long-baseline regime classifier. Returns the mean, variance, and
/// sample stddev, plus the 252-day annualized stddev (using daily
/// log-return would change the definition — this snapshot uses
/// price-level stddev). The `regime_label` compares current N=20
/// stddev against a trailing 60-bar stddev: HIGH_VOL when current
/// >1.5× long, LOW_VOL when <0.67×, MID_VOL otherwise. Distinct
/// from EWMAVOL (exponentially-weighted, ), from REALIZED_VOL
/// (return-based), and from Parkinson/Garman-Klass/RS (range-based).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StddevSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,      // 20
    pub long_length: usize, // 60 — baseline
    pub mean: f64,
    pub variance: f64,
    pub stddev: f64,      // sample stddev of close over N
    pub stddev_long: f64, // sample stddev of close over long_length
    pub cv: f64,          // coefficient of variation = stddev / mean
    pub annualized: f64,  // stddev · sqrt(252)
    pub last_close: f64,
    pub regime_label: String, // HIGH_VOL / MID_VOL / LOW_VOL / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 60 — WMA / RAINBOW / MESA_SINE / FRAMA / IBS ─────────────

/// Weighted Moving Average (WMA) — a linearly-weighted moving
/// average where weights increase from 1 (oldest) to N (newest):
/// `wma = Σ(price[i] · (i+1)) / Σ(i+1)` for i in 0..N-1 with N=20.
/// WMA puts more emphasis on recent bars than SMA but less than
/// EMA, producing a smoother line that still reacts to recent
/// price changes. Distinct from SMA (equal weights), EMA
/// (exponential decay), HMA (WMA of 2·WMA(n/2) − WMA(n)),
/// DEMA/TEMA (EMA recursion), and ALMA (Gaussian kernel).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub wma_value: f64,
    pub wma_prev: f64,
    pub sma_value: f64,
    pub spread: f64,     // close − wma
    pub spread_pct: f64, // (close − wma) / wma
    pub last_close: f64,
    pub wma_label: String, // BULL / WEAK_BULL / NEUTRAL / WEAK_BEAR / BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Rainbow MA Oscillator — a multi-level recursive SMA stack
/// where `r_1 = SMA(close, 2)`, `r_2 = SMA(r_1, 2)`, ..., `r_10 =
/// SMA(r_9, 2)`. The 10 levels create a "rainbow" fan around
/// price, and the oscillator reports the highest-high minus
/// lowest-low across the levels (the rainbow width) along with
/// the fan's current center. A wide rainbow means strong trend
/// (levels spread apart); a narrow rainbow means consolidation
/// (levels bunched tightly). Distinct from GMMA (Guppy's
/// 12-line EMA fan, ) which uses EMAs of varying lengths
/// rather than recursive 2-bar SMAs.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RainbowSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub levels: usize,          // 10
    pub highest_level: f64,     // max across r_1..r_10
    pub lowest_level: f64,      // min across r_1..r_10
    pub rainbow_width: f64,     // highest - lowest
    pub rainbow_width_pct: f64, // width / center
    pub center_value: f64,      // mean of all levels
    pub r1: f64,
    pub r5: f64,
    pub r10: f64,
    pub last_close: f64,
    pub rainbow_label: String, // STRONG_TREND / TRENDING / CONSOLIDATING / INSUFFICIENT_DATA
    pub note: String,
}

/// Ehlers MESA Sine Wave — uses a simplified Hilbert-transform
/// phase estimator to detect the dominant cycle phase and emits
/// `sine = sin(phase)` and `lead_sine = sin(phase + 45°)`. When
/// the sine crosses above the lead_sine, a cycle-bottom buy is
/// flagged; when it crosses below, a cycle-top sell is flagged.
/// In trending markets the two lines separate and fail to cross,
/// producing no signals — a useful regime filter in itself.
/// Distinct from MAMA (phase-adaptive MA, ), FISHER
/// (probability Gaussianization, ), and COG (weighted
/// centroid, ); MESA_SINE focuses on cycle phase rather
/// than value or momentum.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MesaSineSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: f64,     // detected dominant cycle period in bars
    pub phase_rad: f64,  // current phase angle
    pub sine_value: f64, // sin(phase)
    pub lead_sine: f64,  // sin(phase + π/4)
    pub sine_prev: f64,
    pub lead_prev: f64,
    pub last_close: f64,
    pub mesa_label: String, // CYCLE_BUY / CYCLE_SELL / TRENDING / NEUTRAL / INSUFFICIENT_DATA
    pub note: String,
}

/// Fractal Adaptive Moving Average (FRAMA) — Ehlers's adaptive MA
/// where the smoothing α is driven by the fractal dimension D of
/// the price series over the last N bars. Computed by dividing
/// N=16 into two halves, measuring the H-L range of each half
/// and the combined range, then solving for D from the Hurst-like
/// ratio; α = exp(-4.6·(D − 1)). Strong trends (D near 1.0)
/// yield α ≈ 1 (fast-following); choppy markets (D near 2.0)
/// yield α near 0.01 (heavy smoothing). Distinct from KAMA
/// (efficiency-ratio adaptive, ), VIDYA (volatility-index
/// adaptive, ), MAMA (Hilbert-phase adaptive, ),
/// and T3 (Tillson triple-DEMA, ).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FramaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,    // 16
    pub fractal_dim: f64, // D ∈ [1, 2]
    pub alpha: f64,       // exp(-4.6·(D-1))
    pub frama_value: f64,
    pub frama_prev: f64,
    pub spread: f64, // close - frama
    pub last_close: f64,
    pub frama_label: String, // STRONG_TREND / TREND / CHOP / INSUFFICIENT_DATA
    pub note: String,
}

/// Internal Bar Strength (IBS) — the position of close within the
/// current bar's high/low range: `ibs = (close − low) / (high −
/// low)`, bounded on `[0, 1]`. A 14-bar SMA smooths the raw
/// reading. Values near 1 indicate close at the high (bullish
/// conviction within the bar); values near 0 indicate close at
/// the low (bearish conviction). IBS is a mean-reversion favorite
/// — high IBS readings (>0.8) often precede short-term
/// pullbacks, low IBS (<0.2) often precede bounces. Distinct from
/// STOCH (%K over N-bar HHV/LLV, ) which spans multiple
/// bars, and from every momentum oscillator; IBS is a single-bar
/// position metric.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IbsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,     // 14
    pub ibs_raw: f64,      // current bar's IBS
    pub ibs_smoothed: f64, // 14-bar SMA of IBS
    pub ibs_prev: f64,
    pub last_high: f64,
    pub last_low: f64,
    pub last_close: f64,
    pub ibs_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 61 — LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT ──

/// Ehlers Laguerre RSI — a bounded [0, 1] oscillator built from
/// Ehlers's 4-stage Laguerre filter (γ=0.5). The 4-stage filter
/// smooths the close and produces L0, L1, L2, L3 intermediate
/// outputs; the Laguerre RSI is then computed from the count of
/// upward differences vs total differences across the stages,
/// yielding a cleaner oscillator than classic RSI with no
/// divergence false signals near extremes. Distinct from RSI
/// (Wilder smoothing of gains/losses, ), STOCHRSI,
/// CRSI (Connors, ), QQE, and IFT_RSI.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LaguerreRsiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub gamma: f64, // 0.5
    pub l0: f64,
    pub l1: f64,
    pub l2: f64,
    pub l3: f64,
    pub laguerre_rsi: f64, // [0, 1]
    pub laguerre_rsi_prev: f64,
    pub last_close: f64,
    pub lrsi_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// ZigZag pattern detector — reports the most recent confirmed
/// swing high and swing low using a fixed percentage threshold
/// (default 5%). A new pivot forms when price reverses by at least
/// threshold_pct from the prior extreme. The snapshot emits the
/// last high pivot (value + bars_ago), the last low pivot, the
/// active leg direction (UP/DOWN), and the projected reversal
/// level. Distinct from FRACTALS (, Bill Williams 5-bar
/// strict peaks) and from PIVOTS (, prior-session math),
/// which use fundamentally different construction — ZigZag is a
/// %-threshold reversal detector.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ZigzagSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub threshold_pct: f64, // 5.0 (percent)
    pub last_high_value: f64,
    pub last_high_bars_ago: usize,
    pub last_low_value: f64,
    pub last_low_bars_ago: usize,
    pub current_leg: String, // UP / DOWN / INSUFFICIENT_DATA
    pub reversal_level: f64, // threshold from active extreme
    pub last_close: f64,
    pub zigzag_label: String, // UP_LEG / DOWN_LEG / AT_REVERSAL / INSUFFICIENT_DATA
    pub note: String,
}

/// Mark Johnson's Pretty Good Oscillator (PGO) — measures the
/// distance of the current close from an N-period SMA expressed
/// in multiples of the N-period ATR:
/// `pgo = (close − SMA(close, N)) / EMA(TR, N)` with N=14. Extreme
/// readings of ±3 were found to be rare and persistent, making
/// PGO a trend-following signal rather than mean-reversion — the
/// "pretty good" name reflects Johnson's empirical observation that
/// it filters noise better than raw ROC. Distinct from ROC
/// (unscaled price change), PPO (percentage-scaled MACD, ),
/// and RSI/STOCH (bounded oscillators) because PGO's scaling is by
/// volatility, not percent.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PgoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub sma_value: f64,
    pub atr_value: f64, // EMA of TR over N
    pub pgo_value: f64, // (close - sma) / atr
    pub pgo_prev: f64,
    pub last_close: f64,
    pub pgo_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Transform Instantaneous Trendline (HT_TRENDLINE) — a
/// smoothed trendline based on the dominant cycle period derived
/// from Ehlers's Hilbert-transform homodyne discriminator. Unlike
/// MAMA which outputs an adaptive MA proper,
/// HT_TRENDLINE reports the `trendline = WMA(close, period)` over
/// the detected cycle period — a lag-matched smoother that
/// follows the dominant trend without the adaptive α rescaling.
/// Distinct from MAMA (adaptive α), FRAMA (fractal-
/// dimension α), and every fixed-length smoother.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtTrendlineSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: f64, // detected dominant cycle
    pub trendline_value: f64,
    pub trendline_prev: f64,
    pub spread: f64, // close - trendline
    pub spread_pct: f64,
    pub last_close: f64,
    pub ht_label: String, // BULL / WEAK_BULL / NEUTRAL / WEAK_BEAR / BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Midpoint of N — `midpoint = (HHV(N) + LLV(N)) / 2` emitting
/// the midpoint of the N-bar range along with the HHV, LLV, and
/// the close's position within the range. N=14. TA-Lib's MIDPOINT
/// function; useful as a simple anchor for detecting where price
/// sits relative to the most recent trading range. Distinct from
/// Donchian channel (, raw HHV/LLV bands), from SMA, and
/// from pivot systems because it uses only HHV+LLV
/// extremes rather than OHLC4 or session math.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MidpointSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub hhv: f64,
    pub llv: f64,
    pub midpoint: f64, // (HHV + LLV) / 2
    pub midpoint_prev: f64,
    pub close_position: f64, // (close - LLV) / (HHV - LLV), bounded [0, 1]
    pub last_close: f64,
    pub midpoint_label: String, // UPPER / NEAR_UPPER / MIDRANGE / NEAR_LOWER / LOWER / INSUFFICIENT_DATA
    pub note: String,
}

/// Donald Dorsey's Mass Index — an exhaustion/reversal indicator
/// built from a 25-period sum of the ratio of 9-period EMA(H−L) to
/// 9-period EMA of that EMA (EMA of EMA). A classic "reversal
/// bulge" crosses above 27 then below 26.5 signalling a likely
/// reversal independent of direction. Distinct from ATR (range
/// magnitude, ) because Mass Index is a unitless ratio of
/// range smoothings and from CHOP because it measures
/// range expansion/contraction via nested EMAs rather than
/// high-low efficiency.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MassIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_len: usize,     // 9
    pub sum_len: usize,     // 25
    pub ema_range: f64,     // EMA(H-L) latest
    pub ema_ema_range: f64, // EMA of EMA(H-L) latest
    pub ratio: f64,         // ema_range / ema_ema_range
    pub mass_index: f64,    // Σ(ratio) over sum_len
    pub mass_index_prev: f64,
    pub last_close: f64,
    pub mass_label: String, // REVERSAL_BULGE / ELEVATED / NEUTRAL / COMPRESSED / INSUFFICIENT_DATA
    pub note: String,
}

/// Normalized ATR (NATR) — TA-Lib's `natr = 100 × ATR(N) / close`,
/// expressing Wilder's Average True Range as a percentage of the
/// closing price. This makes ATR scale-invariant so it can be
/// compared across symbols of different price levels (a $5 ATR
/// means different things for a $10 stock vs a $500 stock).
/// Distinct from plain ATR (raw dollar volatility, ) and
/// from stddev-based vols (STDDEV) because NATR uses the
/// true range directly rather than log return variance.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NatrSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,   // 14
    pub atr_value: f64,  // Wilder ATR(14)
    pub natr_value: f64, // 100 × ATR / close
    pub natr_prev: f64,
    pub last_close: f64,
    pub natr_label: String, // HIGH_VOL / ELEVATED / NORMAL / LOW_VOL / INSUFFICIENT_DATA
    pub note: String,
}

/// John Carter's TTM Squeeze — a regime flag indicating whether
/// Bollinger Bands (2σ) fit entirely inside Keltner Channels
/// (1.5×ATR). When BB ⊂ KC, volatility is compressed and a
/// breakout is imminent ("squeeze on"). When BB expands outside
/// KC, the squeeze fires ("squeeze off") and directional
/// momentum typically follows. Paired with a linear-regression
/// histogram to indicate breakout direction (up vs down).
/// Distinct from BBW (Bollinger Band Width regime) and
/// from Keltner (standalone bands) because TTM Squeeze
/// tests the geometric relation between both systems.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TtmSqueezeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub bb_upper: f64,
    pub bb_lower: f64,
    pub kc_upper: f64,    // middle + 1.5×ATR
    pub kc_lower: f64,    // middle − 1.5×ATR
    pub squeeze_on: bool, // BB ⊂ KC
    pub momentum: f64,    // linreg of (close - midrange)
    pub momentum_prev: f64,
    pub last_close: f64,
    pub squeeze_label: String, // SQUEEZE_ON / FIRE_UP / FIRE_DOWN / NEUTRAL / INSUFFICIENT_DATA
    pub note: String,
}

/// Alexander Elder's Force Index — `force = volume × (close −
/// close_prev)`, smoothed by a 13-period EMA. Measures the buying
/// / selling pressure behind price moves: strong positive Force
/// means high volume on an up move (bullish conviction); strong
/// negative means heavy selling. Zero-line crossings flag
/// momentum shifts; extreme readings warn of exhaustion.
/// Distinct from OBV (cumulative sign-weighted volume, )
/// because Force Index weights by the size of the price change
/// not just the direction, and from CMF (money-flow-multiplier,
/// ) which uses H/L rather than bar-over-bar close change.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ForceIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,  // 13
    pub force_raw: f64, // latest raw force
    pub force_ema: f64, // EMA-smoothed
    pub force_ema_prev: f64,
    pub last_close: f64,
    pub last_volume: f64,
    pub force_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// True Range (raw, single-bar) — TA-Lib's TRANGE function:
/// `tr = max(H − L, |H − C_prev|, |L − C_prev|)`. The single-bar
/// volatility measure that underlies Wilder's ATR but
/// reports the current bar's TR directly without any smoothing.
/// Useful for gap-aware bar-size comparisons and for building
/// custom volatility systems. Distinct from ATR (N-period EMA of
/// TR) and from the bar's raw range (H − L) which ignores gaps.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TrangeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub trange_value: f64, // latest true range
    pub trange_prev: f64,
    pub mean_trange_20: f64, // 20-bar mean TR
    pub trange_ratio: f64,   // latest / mean_20 (expansion ratio)
    pub last_high: f64,
    pub last_low: f64,
    pub last_close: f64,
    pub prev_close: f64,
    pub trange_label: String, // EXPANSION / NORMAL / CONTRACTION / INSUFFICIENT_DATA
    pub note: String,
}

/// Linear Regression Slope (LINEARREG_SLOPE) — TA-Lib's linreg slope
/// function: the least-squares slope of an N-bar linear regression
/// on close prices, in price-units-per-bar. Distinct from TSF (value
/// of linear regression line at the current bar, ) and from
/// LINEARREG (the regression line value) because this emits just
/// the slope coefficient β. A positive slope indicates trending up;
/// the magnitude captures the rate of trend acceleration.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinearregSlopeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub slope: f64,    // price/bar
    pub slope_prev: f64,
    pub slope_pct: f64, // slope / last_close × 100 (bar-pct)
    pub last_close: f64,
    pub slope_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Dominant Cycle Period (HT_DCPERIOD) — Ehlers's Hilbert-
/// transform homodyne discriminator applied to close prices to
/// detect the dominant cycle period in bars. Distinct from HT_TRENDLINE
/// which uses the period to drive a WMA smoother, and from
/// MESA_SINE / MAMA which use the period for adaptive α rescaling.
/// This snapshot emits just the detected period itself — useful for
/// choosing adaptive parameters on other indicators, or for regime
/// detection where cycle-length dynamics matter.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtDcperiodSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: f64, // dominant cycle in bars (smoothed)
    pub period_prev: f64,
    pub period_min_64: f64, // min over last 64 bars
    pub period_max_64: f64, // max over last 64 bars
    pub last_close: f64,
    pub period_label: String, // VERY_SHORT / SHORT / MEDIUM / LONG / VERY_LONG / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Trend-vs-Cycle Mode (HT_TRENDMODE) — Ehlers's regime
/// classifier derived from the dominant cycle period and discriminator
/// stability: 0 = cycle mode (mean-reverting), 1 = trend mode
/// (directional). Paired with a "lock-in" duration counter showing how
/// many bars the current regime has persisted. Distinct from HT_DCPERIOD
/// which emits the period itself — this emits the binary regime flag.
/// Useful for enabling/disabling mean-reversion vs trend-following
/// strategies in real time.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtTrendmodeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub trendmode: i32, // 0 or 1
    pub trendmode_prev: i32,
    pub lock_in_bars: usize, // how many bars in current mode
    pub period: f64,         // concurrent detected period
    pub last_close: f64,
    pub mode_label: String, // TREND / CYCLE / INSUFFICIENT_DATA
    pub note: String,
}

/// Acceleration Bands (ACCBANDS) — Price Headley's ACCBANDS:
/// `acc_upper = H × (1 + 4×(H−L)/(H+L))`, `acc_lower = L × (1 − 4×(H−L)/(H+L))`,
/// each SMA-smoothed over N=20 periods. The bands are a price-envelope
/// that expands with volatility relative to price level. Breakouts
/// outside the bands signal trend strength. Distinct from Bollinger
/// (σ-based), Keltner (ATR-based), and Donchian (HHV/LLV) because
/// ACCBANDS scales by the range-to-midprice ratio.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AccbandsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub acc_upper: f64,
    pub acc_middle: f64, // SMA of close
    pub acc_lower: f64,
    pub width: f64,    // (upper - lower) / middle
    pub position: f64, // (close - lower) / (upper - lower), [0,1]
    pub last_close: f64,
    pub accbands_label: String, // BREAKOUT_UP / UPPER / MID / LOWER / BREAKOUT_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// Fast Stochastic (STOCHF) — TA-Lib's STOCHF: the unsmoothed
/// stochastic oscillator pair (%K, %D) without the inner 3-bar
/// smoothing of slow STOCH. `%K = 100 × (C − LLV(N)) / (HHV(N) − LLV(N))`
/// with N=14, and %D = SMA(%K, 3). Distinct from STOCH (slow
/// stochastic with inner smoothing applied), STOCHRSI (applied
/// to RSI), and SMI (scaled MIDPRICE). Faster, noisier, more
/// responsive to immediate price action.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StochfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,   // 14
    pub d_period: usize, // 3
    pub fastk: f64,      // 0..100
    pub fastk_prev: f64,
    pub fastd: f64, // SMA(fastk, 3)
    pub fastd_prev: f64,
    pub last_close: f64,
    pub stochf_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// Linear Regression (LINEARREG) — TA-Lib's LINEARREG: the fitted value
/// `y_hat(t) = slope·(N-1) + intercept` of the least-squares regression
/// of close over the last N=14 bars, reporting the endpoint of the
/// fitted line. Distinct from LINEARREG_SLOPE (raw slope),
/// LINEARREG_ANGLE (slope→angle), and LINEARREG_INTERCEPT (y at t=0)
/// because LINEARREG reports the projected endpoint of the line — the
/// most recent fitted value, which is the closest fitted approximation
/// of the current close and a common baseline for mean-reversion
/// signals (close − fitted).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinearregSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub fitted: f64,   // y_hat at current bar
    pub fitted_prev: f64,
    pub residual: f64,     // close - fitted
    pub residual_pct: f64, // residual / close × 100
    pub last_close: f64,
    pub linearreg_label: String, // ABOVE_TREND / NEAR_TREND / BELOW_TREND / INSUFFICIENT_DATA
    pub note: String,
}

/// Linear Regression Angle (LINEARREG_ANGLE) — TA-Lib's
/// LINEARREG_ANGLE: `atan(slope) · 180/π`, converting the raw slope
/// coefficient to an angle in degrees (-90° to +90°). Useful for
/// comparing slope magnitudes across different price scales in a
/// bounded unit. Distinct from LINEARREG_SLOPE (raw units-per-bar)
/// because angle normalizes relative to the price scale.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinearregAngleSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,  // 14
    pub slope: f64,     // raw (price-units per bar)
    pub angle_deg: f64, // atan(slope) · 180/π
    pub angle_deg_prev: f64,
    pub last_close: f64,
    pub angle_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Dominant Cycle Phase (HT_DCPHASE) — TA-Lib's HT_DCPHASE
/// reuses the Ehlers homodyne discriminator pipeline (same as
/// HT_DCPERIOD, ) and reports the phase of the dominant cycle
/// at the current bar in degrees (0..360°). Useful for timing cycle
/// turns — phase 0° is the cycle bottom, 180° is the top. Distinct
/// from HT_DCPERIOD (cycle length in bars) and HT_TRENDMODE (trend
/// vs cycle regime) because it reports the cycle's current position
/// within its rotation rather than its length or regime.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtDcphaseSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub phase_deg: f64, // 0..360°
    pub phase_deg_prev: f64,
    pub phase_delta: f64, // smoothed derivative
    pub period: f64,      // dominant cycle period in bars
    pub last_close: f64,
    pub phase_label: String, // CYCLE_BOTTOM / RISING / CYCLE_TOP / FALLING / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Sine Wave (HT_SINE) — TA-Lib's HT_SINE emits two lines:
/// `sine = sin(phase)` and `leadsine = sin(phase + 45°)`. Crossovers
/// of sine/leadsine identify cycle turns in advance — leadsine
/// crossing above sine signals an imminent cycle bottom, and crossing
/// below signals an imminent top. Distinct from HT_DCPHASE (raw
/// phase) because HT_SINE plots the sine-transformed phase, letting
/// you visualize and cross-trigger cycle turns as bounded signals in
/// [-1, +1].
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtSineSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub sine: f64, // -1..1
    pub sine_prev: f64,
    pub leadsine: f64, // sin(phase + 45°)
    pub leadsine_prev: f64,
    pub crossover: i32, // +1 leadsine above (bottom turn), -1 below (top turn), 0 none
    pub period: f64,
    pub last_close: f64,
    pub sine_label: String, // CYCLE_TURN_UP / BULL / NEUTRAL / BEAR / CYCLE_TURN_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Phasor Components (HT_PHASOR) — TA-Lib's HT_PHASOR emits
/// the raw in-phase (I) and quadrature (Q) components of the
/// analytic signal produced by the Hilbert transform of smoothed
/// price. Magnitude `sqrt(I² + Q²)` is the instantaneous cycle
/// amplitude; `atan2(Q, I)` is the phase. Distinct from HT_DCPHASE
/// (transforms I/Q into the phase angle) and HT_SINE (sine of the
/// phase) because HT_PHASOR reports the raw I/Q components useful
/// for custom cycle analysis and filter design.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtPhasorSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub i_comp: f64, // in-phase component
    pub q_comp: f64, // quadrature component
    pub i_prev: f64,
    pub q_prev: f64,
    pub magnitude: f64, // sqrt(I² + Q²)
    pub phase_deg: f64, // atan2(Q, I) · 180/π
    pub last_close: f64,
    pub phasor_label: String, // STRONG_CYCLE / CYCLE / WEAK_CYCLE / INSUFFICIENT_DATA
    pub note: String,
}

/// MIDPRICE — TA-Lib MIDPRICE function: midpoint between the
/// highest high and the lowest low over an N-bar window (default 14).
/// Distinct from MIDPOINT (close-based midpoint, ) and from
/// Donchian (which exposes both bands separately) because MIDPRICE
/// reports the HH/LL midpoint as a single line anchored to the bar
/// range rather than close.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MidpriceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub midprice: f64, // (HHV(H, 14) + LLV(L, 14)) / 2
    pub midprice_prev: f64,
    pub hhv: f64,
    pub llv: f64,
    pub last_close: f64,
    pub position: f64,          // (close - llv) / (hhv - llv), 0..1
    pub midprice_label: String, // NEAR_HIGH / ABOVE_MID / AT_MID / BELOW_MID / NEAR_LOW / INSUFFICIENT_DATA
    pub note: String,
}

/// APO — TA-Lib Absolute Price Oscillator: `EMA_fast(close)
/// − EMA_slow(close)` with defaults fast=12, slow=26. Distinct from
/// PPO (percentage APO: `(fast − slow) / slow × 100`) and from MACD
/// (APO + signal line + histogram) because APO reports the raw
/// difference in price units, preserving absolute magnitude.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ApoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize, // 12
    pub slow_period: usize, // 26
    pub apo: f64,           // fast_ema - slow_ema
    pub apo_prev: f64,
    pub fast_ema: f64,
    pub slow_ema: f64,
    pub last_close: f64,
    pub apo_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// MOM — TA-Lib raw momentum: `close − close[n−period]`
/// over a 10-bar default lookback. Distinct from ROC (percentage: mom
/// / close[n−period] × 100) and from MOMENTUM_12_1 (composite 12m−1m
/// factor score) because MOM reports the raw price delta in currency
/// units — useful as a pre-scaled input for custom oscillator
/// smoothing or absolute-distance filters.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MomSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 10
    pub mom: f64,      // close - close[n - period]
    pub mom_prev: f64,
    pub mom_pct: f64, // mom / close × 100
    pub last_close: f64,
    pub mom_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// SAREXT — TA-Lib Extended Parabolic SAR with configurable
/// asymmetric long/short acceleration factors and an optional forced
/// start trend. Distinct from PSAR (fixed 0.02/0.02/0.20) in
/// that SAREXT exposes separate af_init/af_step/af_max for long vs
/// short regimes, enabling traders to tune the trailing stop's
/// aggressiveness differently on each side of the trade (typical for
/// instruments with asymmetric volatility).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SarextSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub start_value: f64, // 0 = auto; positive forces start long; negative forces start short
    pub af_init_long: f64, // 0.02
    pub af_step_long: f64, // 0.02
    pub af_max_long: f64, // 0.20
    pub af_init_short: f64, // 0.02
    pub af_step_short: f64, // 0.02
    pub af_max_short: f64, // 0.20
    pub sar_value: f64,
    pub extreme_point: f64,
    pub acceleration_factor: f64,
    pub trend_is_up: bool,
    pub bars_in_trend: usize,
    pub distance_pct: f64,
    pub last_close: f64,
    pub sarext_label: String, // STRONG_UP / UP / STRONG_DOWN / DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// ADXR — TA-Lib Average Directional Movement Rating:
/// `(ADX_now + ADX[n − period]) / 2` over a 14-bar default lookback.
/// Distinct from ADX (point-in-time directional movement strength)
/// because ADXR smooths ADX with its lagged value to emphasise trend
/// persistence — a rising ADXR while ADX is flat signals a maturing
/// trend, while falling ADXR confirms trend exhaustion.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdxrSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub adx_now: f64,
    pub adx_prior: f64, // adx[n - period]
    pub adxr: f64,      // (adx_now + adx_prior) / 2
    pub adxr_prev: f64,
    pub last_close: f64,
    pub adxr_label: String, // STRONG_TREND / TREND / WEAK_TREND / NO_TREND / INSUFFICIENT_DATA
    pub note: String,
}
