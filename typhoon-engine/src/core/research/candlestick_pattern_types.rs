// Candlestick pattern research types

/// candlestick patternPIERCING — Piercing Line (2-bar bullish reversal, mirror
/// of Dark Cloud Cover). Prior bar red with large body; current bar
/// green, opens below prior low, closes above prior midpoint (≥ 50%
/// penetration). Emits +100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlPiercingSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub penetration_pct: f64, // 100 · (current_close - prior_close) / prior_body
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_piercing_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternDRAGONFLYDOJI — Dragonfly Doji (single-bar support
/// signal). Doji body (≤ 5% of range) with open ≈ high ≈ close
/// and long lower shadow. T-shape indicating rejection of lower
/// prices. TA-Lib emits +100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlDragonflyDojiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64, // % of range above body
    pub lower_shadow_pct: f64, // % of range below body (dominant for dragonfly)
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_dragonfly_doji_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternGRAVESTONEDOJI — Gravestone Doji (single-bar resistance
/// signal). Doji body (≤ 5% of range) with open ≈ low ≈ close and
/// long upper shadow. Inverted-T shape indicating rejection of
/// higher prices. TA-Lib emits -100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlGravestoneDojiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64, // dominant for gravestone
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_gravestone_doji_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternHANGINGMAN — Hanging Man (single-bar bearish reversal
/// at tops). Geometrically identical to the Hammer but appearing at
/// market tops instead of bottoms: small body in the upper third,
/// long lower shadow ≥ 2× body, minimal upper shadow. TA-Lib emits
/// -100 on match (sign-flipped from Hammer's +100 to signal bearish
/// top context vs. bullish bottom context).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHangingManSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64, // dominant (≥ 2× body)
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_hanging_man_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternINVERTEDHAMMER — Inverted Hammer (single-bar bullish
/// reversal at bottoms). Mirror of Shooting Star but appearing at
/// bottoms instead of tops: small body in the lower third, long
/// upper shadow ≥ 2× body, minimal lower shadow. TA-Lib emits
/// +100 on match (sign-flipped from Shooting Star's -100).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlInvertedHammerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64, // dominant (≥ 2× body)
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_inverted_hammer_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternHARAMICROSS — Harami Cross (stricter 2-bar reversal).
/// Variant of Harami where the inside bar is a doji (body ≤ 5% of
/// range) rather than any small opposite-direction body. TA-Lib
/// treats this as a more potent reversal signal than regular
/// Harami; emits +100 (bullish) when prior bar is red and current
/// is a doji contained in prior body; -100 (bearish) mirror.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHaramiCrossSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64, // must be ≤ 5% to qualify as doji
    pub body_size_ratio: f64,        // cur_body / prior_body, always < 1.0 when match
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_harami_cross_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternLONGLEGGEDDOJI — Long-Legged Doji (single-bar
/// indecision with wide range). Doji body (≤ 5% of range) with
/// BOTH upper and lower shadows dominant (each ≥ 30% of range).
/// Signals strong indecision after a meaningful price excursion
/// in both directions within the bar. TA-Lib emits +100 on match
/// (treated as directionally neutral like regular doji; context
/// determines bullish/bearish implication).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlLongLeggedDojiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 pattern present, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,   // ≤ 5%
    pub upper_shadow_pct: f64, // ≥ 30%
    pub lower_shadow_pct: f64, // ≥ 30%
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_long_legged_doji_label: String, // DOJI_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternMARUBOZU — Marubozu (single-bar pure-body
/// conviction). Bar with little to no shadows (each ≤ 5% of
/// range) and body ≥ 90% of range. Bullish marubozu = green
/// (open == low, close == high), bearish marubozu = red
/// (open == high, close == low). Strongest single-bar
/// directional conviction signal. TA-Lib emits +100 (bullish)
/// or -100 (bearish).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlMarubozuSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,   // ≥ 90%
    pub upper_shadow_pct: f64, // ≤ 5%
    pub lower_shadow_pct: f64, // ≤ 5%
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_marubozu_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternSPINNINGTOP — Spinning Top (single-bar indecision
/// with moderate shadows). Small body (≤ 30% of range) centred in
/// range with BOTH upper and lower shadows larger than the body.
/// Indicates indecision but less extreme than long-legged doji.
/// TA-Lib emits +100 (green body) or -100 (red body) though both
/// are treated as indecision signals regardless of body colour.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlSpinningTopSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 green-body, -100 red-body, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,   // ≤ 30%
    pub upper_shadow_pct: f64, // > body
    pub lower_shadow_pct: f64, // > body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_spinning_top_label: String, // GREEN_BODY_PATTERN / RED_BODY_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternTRISTAR — Tri-Star (3-bar rare triple-doji
/// reversal). Three consecutive doji bars (each body ≤ 5% of
/// range). TA-Lib emits +100 (bullish) when middle doji gaps
/// below the outer two and the third doji closes above the
/// middle; -100 (bearish) mirror. Rare but high-conviction
/// reversal signal when present.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlTristarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub avg_body_pct_range: f64, // average across the three dojis
    pub middle_gap_pct: f64,     // signed % gap of middle doji from outer two
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_tristar_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternDOJISTAR — Doji Star (2-bar reversal precursor).
/// Prior bar has a real body (body ≥ 30% of range), current bar
/// is a doji (body ≤ 5% of range) that gaps away from the prior
/// close. -100 (bearish) when prior is green and current doji
/// gaps above prior close; +100 (bullish) when prior is red and
/// current doji gaps below prior close. Precursor to the full
/// 3-bar MORNINGDOJISTAR / EVENINGDOJISTAR patterns.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlDojiStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,   // ≥ 30% to qualify as real body
    pub current_body_pct_range: f64, // ≤ 5% to qualify as doji
    pub gap_pct: f64,                // signed % gap from prior close
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_doji_star_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternMORNINGDOJISTAR — Morning Doji Star (3-bar bullish
/// reversal, doji-middle variant of R73 MORNINGSTAR). Bar 1 is a
/// long red body (≥ 30% of range); bar 2 is a doji (body ≤ 5%)
/// that gaps below bar 1's close; bar 3 is green and closes above
/// bar 1's midpoint. Stronger bullish-reversal conviction than
/// regular morning star because the doji indicates explicit
/// equilibrium after the sell-off before the recovery.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlMorningDojiStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub bar1_body_pct_range: f64,        // ≥ 30%
    pub bar2_body_pct_range: f64,        // doji ≤ 5%
    pub bar3_close_vs_bar1_mid_pct: f64, // signed % above (positive) bar 1 midpoint
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_morning_doji_star_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternEVENINGDOJISTAR — Evening Doji Star (3-bar bearish
/// reversal, doji-middle variant of R73 EVENINGSTAR). Bar 1 is a
/// long green body; bar 2 is a doji that gaps above bar 1's close;
/// bar 3 is red and closes below bar 1's midpoint. Stronger
/// bearish-reversal conviction than regular evening star because
/// the doji indicates explicit equilibrium after the rally before
/// the breakdown.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlEveningDojiStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub bar1_body_pct_range: f64,        // ≥ 30%
    pub bar2_body_pct_range: f64,        // doji ≤ 5%
    pub bar3_close_vs_bar1_mid_pct: f64, // signed % below (negative) bar 1 midpoint
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_evening_doji_star_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternABANDONEDBABY — Abandoned Baby (strongest 3-bar star
/// variant). Doji "abandoned" by full-body-and-shadow gaps on both
/// sides. Bullish: bar 1 long red, bar 2 doji with bar2.high <
/// bar1.low (no overlap), bar 3 green with bar3.low > bar2.high
/// (full gap away). Bearish: mirror. Rare but very high-conviction
/// reversal signal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlAbandonedBabySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub bar1_body_pct_range: f64, // ≥ 30%
    pub bar2_body_pct_range: f64, // doji ≤ 5%
    pub gap_down_pct: f64,        // signed % gap between bar 1 low and bar 2 high
    pub gap_up_pct: f64,          // signed % gap between bar 2 high and bar 3 low
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_abandoned_baby_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick pattern3INSIDE — Three Inside Up/Down (confirmed Harami).
/// Bar 1 has a long body; bar 2 is a small body of the opposite
/// colour fully contained within bar 1's body (Harami geometry);
/// bar 3 closes beyond bar 1's body in the direction opposite to
/// bar 1 (confirmation). Bullish: bar 1 red + bar 2 small green
/// inside + bar 3 closes above bar 1's open. Bearish: mirror.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeInsideSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub bar1_body_pct_range: f64,         // ≥ 30%
    pub body_size_ratio: f64,             // bar 2 body / bar 1 body, < 1.0 when match
    pub bar3_close_vs_bar1_open_pct: f64, // signed % distance from bar 1 open
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_inside_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 77 — CDLBELTHOLD / CDLCLOSINGMARUBOZU / CDLHIGHWAVE /
//    CDLLONGLINE / CDLSHORTLINE ──

/// candlestick patternBELTHOLD — Belt-hold line. Long real body with virtually
/// no opening shadow. Bullish when a green candle opens at/near the low
/// of the range; bearish when a red candle opens at/near the high.
/// Strong single-bar conviction pattern. TA-Lib emits +100 / -100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlBeltHoldSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,     // long body threshold
    pub opening_shadow_pct: f64, // lower shadow for green, upper for red
    pub closing_shadow_pct: f64, // opposite-side shadow
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_belt_hold_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternCLOSINGMARUBOZU — Closing Marubozu. Long real body with
/// virtually no closing shadow. Bullish when a green candle closes at/
/// near the high; bearish when a red candle closes at/near the low.
/// TA-Lib emits +100 / -100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlClosingMarubozuSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,     // long body threshold
    pub opening_shadow_pct: f64, // lower shadow for green, upper for red
    pub closing_shadow_pct: f64, // upper shadow for green, lower for red
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_closing_marubozu_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternHIGHWAVE — High-Wave Candle. Small body but long shadows
/// on both sides, signalling strong intrabar indecision with large
/// excursion in both directions. TA-Lib emits +100 for green body,
/// -100 for red body.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHighWaveSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 green-body, -100 red-body, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,   // small body
    pub upper_shadow_pct: f64, // long upper shadow
    pub lower_shadow_pct: f64, // long lower shadow
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_high_wave_label: String, // GREEN_BODY_PATTERN / RED_BODY_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternLONGLINE — Long Line Candle. Long real body with relatively
/// small shadows at both ends. TA-Lib emits +100 for green body and
/// -100 for red body.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlLongLineSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 green-body, -100 red-body, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64, // dominant body
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_long_line_label: String, // GREEN_BODY_PATTERN / RED_BODY_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternSHORTLINE — Short Line Candle. Short real body with
/// relatively small shadows. TA-Lib emits +100 for green body and
/// -100 for red body.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlShortLineSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 green-body, -100 red-body, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64, // short body
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_short_line_label: String, // GREEN_BODY_PATTERN / RED_BODY_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 78 — CDLCOUNTERATTACK / CDLHOMINGPIGEON / CDLINNECK /
//    CDLONNECK / CDLTHRUSTING ──

/// candlestick patternCOUNTERATTACK — Counterattack lines. Two opposite-colour
/// long candles with a gap open in the direction of the prior bar, then
/// a close back at/near the prior close. Bullish when the first bar is
/// red and the second green; bearish mirror for green→red.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlCounterattackSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub gap_open_pct: f64,        // positive in the direction of the gap
    pub close_diff_pct_body: f64, // abs close-vs-prior-close difference as % of prior body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_counterattack_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternHOMINGPIGEON — Homing Pigeon. A bearish harami variant:
/// long red body followed by a smaller red body fully inside the first.
/// Bullish reversal pattern in TA-Lib, emitting +100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHomingPigeonSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub body_size_ratio: f64,       // current / prior
    pub inner_body_margin_pct: f64, // min inner-body clearance as % of prior body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_homing_pigeon_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternINNECK — In-Neck pattern. Long red body followed by a
/// green candle that gaps below the prior low and closes slightly into
/// the prior real body. Bearish continuation, TA-Lib emits -100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlInNeckSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub gap_open_pct: f64,
    pub penetration_pct: f64, // close into prior body as % of prior body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_in_neck_label: String, // BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternONNECK — On-Neck pattern. Long red body followed by a
/// green candle that gaps below the prior low and closes back at/near
/// the prior close. Bearish continuation, TA-Lib emits -100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlOnNeckSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub gap_open_pct: f64,
    pub close_match_pct: f64, // abs close-vs-prior-close difference as % of prior body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_on_neck_label: String, // BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternTHRUSTING — Thrusting pattern. Long red body followed by a
/// green candle that gaps below the prior low and closes into the prior
/// body, but not as deep as the midpoint. Bearish continuation, emits
/// -100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThrustingSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub gap_open_pct: f64,
    pub penetration_pct: f64, // deeper than in-neck but below midpoint
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_thrusting_label: String, // BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 79 — CDL2CROWS / CDL3LINESTRIKE / CDL3OUTSIDE /
//    CDLMATCHINGLOW ──

/// candlestick pattern2CROWS — Two Crows. Long green body, then a gap-up red
/// candle, followed by another red candle that opens inside the second
/// body and closes back inside the first real body. Bearish reversal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlTwoCrowsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_gap_pct: f64,        // gap-up of bar 2 body vs bar 1 body
    pub third_penetration_pct: f64, // bar 3 close into bar 1 body as % of bar 1 body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_two_crows_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick pattern3LINESTRIKE — Three Line Strike. Three same-direction
/// candles followed by a large opposite-colour strike candle that
/// closes beyond the first bar's open. Reversal signal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeLineStrikeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub avg_first_three_body_pct_range: f64,
    pub strike_body_pct_range: f64,
    pub strike_close_vs_first_open_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_line_strike_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick pattern3OUTSIDE — Three Outside Up/Down. An engulfing reversal
/// confirmed by a third candle continuing in the same direction.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeOutsideSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub engulf_body_ratio: f64,      // bar2 body / bar1 body
    pub confirmation_pct_body2: f64, // bar3 close extension beyond bar2 close as % of bar2 body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_outside_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternMATCHINGLOW — Matching Low. Two red candles that close at
/// nearly the same level, signalling potential support. Bullish in
/// TA-Lib (+100).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlMatchingLowSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub close_match_pct_body: f64, // abs(close2-close1) as % of first body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_matching_low_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 80 — CDLSEPARATINGLINES / CDLSTICKSANDWICH /
//    CDLRICKSHAWMAN / CDLTAKURI ──

/// candlestick patternSEPARATINGLINES — Separating Lines. Opposite-colour
/// candles with the same open, where the second resumes the prevailing
/// direction. Continuation pattern with both bullish and bearish forms.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlSeparatingLinesSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub open_match_pct_body: f64, // abs(open2-open1) as % of first body
    pub continuation_pct_body: f64, // close extension beyond first open as % of first body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_separating_lines_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternSTICKSANDWICH — Stick Sandwich. Red / green / red where
/// the first and third closes match, marking potential support.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlStickSandwichSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub close_match_pct_body: f64,
    pub middle_rebound_pct: f64, // middle close above first close as % of first body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_stick_sandwich_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternRICKSHAWMAN — Rickshaw Man. A centered doji with long
/// upper and lower shadows. Neutral indecision pattern, reported here
/// as +100 when present for parity/discoverability.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlRickshawManSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 present, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub body_midpoint_offset_pct: f64, // distance of body midpoint from range midpoint
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_rickshaw_man_label: String, // RICKSHAW_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternTAKURI — Takuri. Dragonfly-like doji with an especially
/// long lower shadow. Bullish reversal variant.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlTakuriSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub lower_to_upper_ratio: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_takuri_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// Candlestick pattern storage/helpers

/// candlestick pattern3STARSINSOUTH — Three Stars in the South. A 3-bar bullish
/// reversal made of three descending red candles where downside pressure
/// progressively contracts: a long black candle with a long lower shadow,
/// then a smaller black candle with a higher low, then a small black bar
/// nested inside the second bar.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeStarsInSouthSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub first_lower_shadow_pct: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub third_inside_pct_range: f64, // how deeply bar 3 sits inside bar 2 range
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_stars_in_south_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternIDENTICAL3CROWS — Identical Three Crows. Bearish 3-bar
/// continuation: three long red candles with each new candle opening
/// near the prior close and extending the decline.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlIdenticalThreeCrowsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub avg_body_pct_range: f64,
    pub open1_vs_close0_pct_body: f64, // abs(open2-close1) as % of body1
    pub open2_vs_close1_pct_body: f64, // abs(open3-close2) as % of body2
    pub total_close_decline_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_identical_three_crows_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternKICKING — Kicking. Two opposite-colour marubozu candles
/// separated by a clean gap between their full ranges.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlKickingSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub gap_pct_range: f64, // gap magnitude as % of first bar range
    pub second_to_first_body_ratio: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_kicking_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternKICKINGBYLENGTH — Kicking where bull/bear direction is
/// assigned from the longer of the two marubozu bodies.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlKickingByLengthSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub gap_pct_range: f64,
    pub dominant_body_ratio: f64, // larger body / smaller body
    pub dominant_side: String,    // FIRST_BAR / SECOND_BAR / NONE
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_kicking_by_length_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternLADDERBOTTOM — Ladder Bottom. Five-bar bullish reversal:
/// three descending red candles, a fourth red "rung" with an upper
/// shadow, then a strong green breakout candle.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlLadderBottomSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub avg_first_three_body_pct_range: f64,
    pub fourth_body_pct_range: f64,
    pub fourth_upper_shadow_pct: f64,
    pub fifth_body_pct_range: f64,
    pub breakout_pct_vs_fourth_high: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_ladder_bottom_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternUNIQUE3RIVER — Unique 3 River. Three-bar bullish reversal:
/// long red candle, then a smaller red candle with an extended lower
/// shadow, followed by a small green candle tucked inside the second.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlUniqueThreeRiverSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub second_lower_shadow_pct: f64,
    pub third_body_pct_range: f64,
    pub third_close_vs_second_close_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_unique_three_river_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// Candlestick pattern storage/helpers

/// candlestick patternADVANCEBLOCK — Advance Block. Three rising green candles
/// whose progress weakens as bodies shrink and upper shadows lengthen.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlAdvanceBlockSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub third_upper_shadow_pct: f64,
    pub total_close_gain_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_advance_block_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternBREAKAWAY — Breakaway. Five-bar reversal pattern with an
/// initial gap in trend direction and a final candle that closes back
/// into that gap.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlBreakawaySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub initial_gap_pct_range: f64,
    pub fifth_body_pct_range: f64,
    pub gap_retracement_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_breakaway_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternGAPSIDESIDEWHITE — Up/Down-gap side-by-side white lines.
/// Two similar green candles that hold a gap versus the prior candle.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlGapSideSideWhiteSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish continuation, -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub gap_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub open_similarity_pct_body: f64,
    pub close_similarity_pct_body: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_gap_side_side_white_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternUPSIDEGAP2CROWS — Upside Gap Two Crows. Long green candle,
/// gap-up red candle, then another red candle that opens higher and
/// closes back into the gap.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlUpsideGapTwoCrowsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub upside_gap_pct_range: f64,
    pub third_open_above_second_pct_body: f64,
    pub third_close_into_gap_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_upside_gap_two_crows_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternXSIDEGAP3METHODS — Upside/Downside Gap Three Methods.
/// Two same-direction candles gap away from the first, then an
/// opposite-colour candle closes into that gap without fully reversing it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlXSideGapThreeMethodsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish continuation, -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub gap_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub gap_fill_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_xside_gap_three_methods_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternCONCEALBABYSWALL — Concealing Baby Swallow. Four black
/// candles where the first two are marubozu-like, the third gaps down,
/// and the fourth engulfs the third candle's range.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlConcealBabySwallowSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish reversal, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_upper_shadow_pct: f64,
    pub fourth_range_engulf_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_conceal_baby_swallow_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// Candlestick pattern storage/helpers

/// candlestick patternHIKKAKE — Hikkake. Inside-bar setup followed by a false
/// break to one side of the inside bar.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHikkakeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub inside_width_pct_mother: f64,
    pub false_break_extension_pct: f64,
    pub trigger_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_hikkake_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternHIKKAKEMOD — Modified Hikkake. Hikkake-like inside-bar
/// trap followed by an explicit confirmation bar.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHikkakeModSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub inside_width_pct_mother: f64,
    pub false_break_extension_pct: f64,
    pub confirmation_extension_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_hikkake_mod_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternMATHOLD — Mat Hold. A strong trend candle, a gapped pause,
/// two holding candles, then a continuation breakout.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlMatHoldSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish continuation, -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub middle_avg_body_pct_range: f64,
    pub initial_gap_pct_range: f64,
    pub hold_depth_pct_body: f64,
    pub final_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_mat_hold_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternRISEFALL3METHODS — Rising/Falling Three Methods. Long
/// trend candle, three small counter-trend candles inside it, then a
/// continuation candle in the original direction.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlRiseFallThreeMethodsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish continuation, -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub middle_avg_body_pct_range: f64,
    pub containment_pct_body: f64,
    pub final_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_rise_fall_three_methods_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// Candlestick pattern storage/helpers

/// candlestick patternSTALLEDPATTERN — Stalled Pattern. Three advancing white
/// candles where the third gaps up but loses momentum with a small real
/// body and meaningful upper shadow.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlStalledPatternSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish reversal, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub third_open_gap_pct_range: f64,
    pub third_upper_shadow_pct: f64,
    pub close_progress_pct_prev_leg: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_stalled_pattern_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternTASUKIGAP — Tasuki Gap. Two same-direction candles with a
/// trend gap, followed by an opposite-colour candle that retraces into
/// the gap without fully closing it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlTasukiGapSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish continuation, -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub gap_pct_range: f64,
    pub gap_fill_pct: f64,
    pub third_open_pct_second_body: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_tasuki_gap_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}
