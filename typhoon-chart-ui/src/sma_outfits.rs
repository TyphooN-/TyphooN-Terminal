//! SMA-outfit intelligence.
//!
//! Concept credit: **"SMA Outfits"** by raultrades / Unfair Market
//! (github.com/raultrades/SMA-outfits) — predetermined sets of SMA periods
//! treated as one institutional execution-trigger system on liquid equities —
//! and the Apache-2.0 **sma-intelligence-platform** (niya-shroff), which
//! layers multi-outfit signal generation with confidence metrics on top.
//!
//! This module is TyphooN's own implementation of the idea: pure,
//! deterministic bar math (no network, no ML, no code shared with either
//! project). Per outfit it reports the stack state, price/SMA geometry,
//! most-recent price↔SMA crosses, trigger-band proximity, and a pairwise
//! alignment percentage that serves as the outfit's confidence score.

use crate::indicators::compute_sma;
use crate::types::Bar;

/// Outfit periods are held to the SMA-outfits spec: 1..=999.
pub const SMA_OUTFIT_MIN_PERIOD: usize = 1;
pub const SMA_OUTFIT_MAX_PERIOD: usize = 999;
/// Keep an outfit readable: 2..=6 SMAs.
pub const SMA_OUTFIT_MAX_LEGS: usize = 6;

/// The two canonical outfits from the SMA-intelligence lineage.
pub fn default_sma_outfits() -> Vec<Vec<usize>> {
    vec![vec![10, 50, 200], vec![30, 60, 90]]
}

/// How price currently "wears" the outfit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutfitStack {
    /// price > shortest SMA > … > longest SMA (fully dressed bullish).
    Bullish,
    /// price < shortest SMA < … < longest SMA (fully dressed bearish).
    Bearish,
    /// Anything in between.
    Mixed,
}

impl OutfitStack {
    pub fn label(self) -> &'static str {
        match self {
            OutfitStack::Bullish => "BULLISH STACK",
            OutfitStack::Bearish => "BEARISH STACK",
            OutfitStack::Mixed => "MIXED",
        }
    }
}

/// Per-SMA leg state within an outfit.
#[derive(Debug, Clone)]
pub struct OutfitLeg {
    pub period: usize,
    pub value: f64,
    /// Signed distance of the last close from this SMA, in percent.
    pub price_delta_pct: f64,
    /// |price_delta_pct| ≤ trigger band — price is sitting on an
    /// institutional trigger level.
    pub at_trigger: bool,
    /// Bars since the close most recently crossed this SMA (0 = crossed on
    /// the latest bar), if a cross exists within the scanned lookback.
    pub bars_since_cross: Option<usize>,
    /// Direction of that most recent cross (true = crossed up).
    pub last_cross_up: bool,
}

/// Full analysis of one outfit over a bar series.
#[derive(Debug, Clone)]
pub struct OutfitReport {
    pub periods: Vec<usize>,
    pub legs: Vec<OutfitLeg>,
    pub stack: OutfitStack,
    /// Percentage (0–100) of pairwise bullish relations that hold:
    /// price>SMA per leg plus shorter-SMA>longer-SMA per adjacent pair.
    /// 100 ⇔ Bullish stack, 0 ⇔ Bearish stack.
    pub alignment_pct: f64,
    /// True when the series is shorter than the longest period — the report
    /// covers only the legs that had enough bars.
    pub insufficient_history: bool,
}

/// Parse a user outfit spec like `"10/50/200"` or `"10, 50, 200"`.
/// Returns ascending, deduplicated periods; None when empty, out of the
/// 1..=999 spec range, or more than [`SMA_OUTFIT_MAX_LEGS`] legs.
pub fn parse_outfit_spec(spec: &str) -> Option<Vec<usize>> {
    let mut periods: Vec<usize> = Vec::new();
    for token in spec.split(['/', ',', ' ']) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let p: usize = token.parse().ok()?;
        if !(SMA_OUTFIT_MIN_PERIOD..=SMA_OUTFIT_MAX_PERIOD).contains(&p) {
            return None;
        }
        periods.push(p);
    }
    periods.sort_unstable();
    periods.dedup();
    if periods.len() < 2 || periods.len() > SMA_OUTFIT_MAX_LEGS {
        return None;
    }
    Some(periods)
}

/// Render an outfit's periods back to the canonical `a/b/c` form.
pub fn outfit_label(periods: &[usize]) -> String {
    periods
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// Analyze one outfit. `trigger_band_pct` is the ± band (in percent of the
/// SMA value) inside which price counts as sitting on the trigger;
/// `cross_lookback` bounds the most-recent-cross scan.
pub fn analyze_sma_outfit(
    bars: &[Bar],
    periods: &[usize],
    trigger_band_pct: f64,
    cross_lookback: usize,
) -> OutfitReport {
    let mut sorted: Vec<usize> = periods.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    let last_close = bars.last().map(|b| b.close).unwrap_or(f64::NAN);
    let mut legs: Vec<OutfitLeg> = Vec::with_capacity(sorted.len());
    let mut insufficient = false;

    for &period in &sorted {
        let series = compute_sma(bars, period);
        let Some(value) = series.last().copied().flatten() else {
            insufficient = true;
            continue;
        };
        if value <= 0.0 || !value.is_finite() || !last_close.is_finite() {
            insufficient = true;
            continue;
        }
        let price_delta_pct = (last_close - value) / value * 100.0;

        // Most recent close↔SMA cross within the lookback: walk back through
        // consecutive (close - sma) sign pairs.
        let mut bars_since_cross = None;
        let mut last_cross_up = false;
        let n = bars.len();
        let scan = cross_lookback.min(n.saturating_sub(1));
        for back in 0..scan {
            let idx_new = n - 1 - back;
            let idx_old = idx_new - 1;
            let (Some(sma_new), Some(sma_old)) = (series[idx_new], series[idx_old]) else {
                break; // ran out of SMA history
            };
            let above_new = bars[idx_new].close > sma_new;
            let above_old = bars[idx_old].close > sma_old;
            if above_new != above_old {
                bars_since_cross = Some(back);
                last_cross_up = above_new;
                break;
            }
        }

        legs.push(OutfitLeg {
            period,
            value,
            price_delta_pct,
            at_trigger: price_delta_pct.abs() <= trigger_band_pct,
            bars_since_cross,
            last_cross_up,
        });
    }

    // Pairwise relations: price vs every leg + each adjacent shorter/longer
    // ordering. Exact ties are neutral — price sitting ON an SMA is neither
    // side of the trigger, so alignment reads 50 for a perfectly flat tape
    // (100 = fully dressed bullish, 0 = fully dressed bearish).
    let mut relations = 0usize;
    let mut bullish = 0usize;
    let mut bearish = 0usize;
    for leg in &legs {
        relations += 1;
        if last_close > leg.value {
            bullish += 1;
        } else if last_close < leg.value {
            bearish += 1;
        }
    }
    for pair in legs.windows(2) {
        relations += 1;
        if pair[0].value > pair[1].value {
            bullish += 1;
        } else if pair[0].value < pair[1].value {
            bearish += 1;
        }
    }
    let alignment_pct = if relations == 0 {
        50.0
    } else {
        let ties = relations - bullish - bearish;
        (bullish as f64 + ties as f64 * 0.5) / relations as f64 * 100.0
    };
    let stack = if relations == 0 || legs.len() < sorted.len() {
        OutfitStack::Mixed
    } else if bullish == relations {
        OutfitStack::Bullish
    } else if bearish == relations {
        OutfitStack::Bearish
    } else {
        OutfitStack::Mixed
    };

    OutfitReport {
        periods: sorted,
        legs,
        stack,
        alignment_pct,
        insufficient_history: insufficient,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars_from_closes(closes: &[f64]) -> Vec<Bar> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &c)| Bar {
                ts_ms: i as i64 * 60_000,
                open: c,
                high: c,
                low: c,
                close: c,
                volume: 1.0,
            })
            .collect()
    }

    #[test]
    fn parse_outfit_spec_accepts_slash_comma_space_and_sorts() {
        assert_eq!(parse_outfit_spec("10/50/200"), Some(vec![10, 50, 200]));
        assert_eq!(parse_outfit_spec("200, 10, 50"), Some(vec![10, 50, 200]));
        assert_eq!(parse_outfit_spec("30 60 90"), Some(vec![30, 60, 90]));
        assert_eq!(parse_outfit_spec("10/10/50"), Some(vec![10, 50]));
    }

    #[test]
    fn parse_outfit_spec_enforces_spec_bounds() {
        assert_eq!(parse_outfit_spec("0/50"), None, "below spec minimum");
        assert_eq!(parse_outfit_spec("10/1000"), None, "above spec maximum");
        assert_eq!(parse_outfit_spec("10"), None, "single leg is not an outfit");
        assert_eq!(parse_outfit_spec(""), None);
        assert_eq!(parse_outfit_spec("1/2/3/4/5/6/7"), None, "too many legs");
        assert_eq!(parse_outfit_spec("abc/50"), None);
    }

    #[test]
    fn rising_series_is_fully_dressed_bullish() {
        // Strictly rising closes: price > SMA(short) > SMA(long) everywhere.
        let closes: Vec<f64> = (1..=60).map(|i| i as f64).collect();
        let report = analyze_sma_outfit(&bars_from_closes(&closes), &[5, 20], 0.5, 10);
        assert_eq!(report.stack, OutfitStack::Bullish);
        assert_eq!(report.alignment_pct, 100.0);
        assert!(!report.insufficient_history);
        assert_eq!(report.legs.len(), 2);
        assert!(report.legs[0].price_delta_pct > 0.0);
    }

    #[test]
    fn falling_series_is_fully_dressed_bearish() {
        let closes: Vec<f64> = (1..=60).map(|i| (100 - i) as f64).collect();
        let report = analyze_sma_outfit(&bars_from_closes(&closes), &[5, 20], 0.5, 10);
        assert_eq!(report.stack, OutfitStack::Bearish);
        assert_eq!(report.alignment_pct, 0.0);
    }

    #[test]
    fn cross_detection_reports_bars_since_and_direction() {
        // Flat at 10 long enough to pin SMA(4)≈10, then jump above: the close
        // crosses up over the SMA on the jump bar.
        let mut closes = vec![10.0; 20];
        closes.push(20.0); // cross up happens here
        closes.push(21.0);
        closes.push(22.0);
        let report = analyze_sma_outfit(&bars_from_closes(&closes), &[2, 4], 0.5, 10);
        let leg4 = report.legs.iter().find(|l| l.period == 4).unwrap();
        assert_eq!(leg4.bars_since_cross, Some(2), "crossed 2 bars back");
        assert!(leg4.last_cross_up);
    }

    #[test]
    fn trigger_band_flags_price_on_the_sma() {
        // Constant series: price == SMA exactly → delta 0% → at trigger.
        let closes = vec![50.0; 30];
        let report = analyze_sma_outfit(&bars_from_closes(&closes), &[5, 10], 0.5, 10);
        assert!(report.legs.iter().all(|l| l.at_trigger));
        assert_eq!(report.stack, OutfitStack::Mixed, "flat is neither stack");
        assert_eq!(report.alignment_pct, 50.0, "all-ties tape reads neutral");
    }

    #[test]
    fn short_history_reports_insufficient_and_partial_legs() {
        let closes: Vec<f64> = (1..=30).map(|i| i as f64).collect();
        let report = analyze_sma_outfit(&bars_from_closes(&closes), &[10, 200], 0.5, 10);
        assert!(report.insufficient_history);
        assert_eq!(report.legs.len(), 1, "only the 10-period leg fits");
        assert_eq!(report.stack, OutfitStack::Mixed, "partial outfit never claims a stack");
    }

    #[test]
    fn outfit_label_is_canonical_slash_form() {
        assert_eq!(outfit_label(&[10, 50, 200]), "10/50/200");
    }
}
