//! Repainting diagnostic for indicators (ADR-135 §11.5).
//!
//! An indicator repaints when a value it already published for a *closed* bar
//! changes because later bars arrived. That is the difference between a
//! backtest that could have been traded and one that could not, and it is
//! invisible to any test that only inspects the final series.
//!
//! The method is the one §11.5 specifies: evaluate the indicator over every
//! prefix of the bar series, snapshot what was visible after each event, and
//! report every value that later moved. A finding names the exact output, the
//! bar whose value changed, the event that changed it, and both values — never
//! a single "this indicator repaints" flag, which tells an author nothing about
//! where to look.
//!
//! # Declared revision windows
//!
//! Some indicators legitimately revise recent output — a centred average or a
//! confirmed swing point cannot be final on the bar it first appears. That is
//! honest only if it is *declared*: [`RepaintPolicy::revision_window_bars`] is
//! the number of trailing bars an indicator is allowed to rewrite. A change
//! inside the window is expected behaviour; a change outside it is a finding.
//! An undeclared window is zero, so silence is the strict setting.

use crate::core::strategy_simulator::SimBar;
use serde::{Deserialize, Serialize};

/// Wire-format version of [`RepaintReport`].
pub const REPAINT_REPORT_SCHEMA_VERSION: u32 = 1;

/// Largest bar series this diagnostic will scan. Evaluation is quadratic in the
/// bar count by construction — it re-evaluates every prefix — so the bound is
/// what keeps a diagnostic from becoming a denial of service.
pub const MAX_REPAINT_BARS: usize = 4_096;

/// Largest number of outputs one indicator may declare.
pub const MAX_REPAINT_OUTPUTS: usize = 64;

/// What an indicator is allowed to do to its own history.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepaintPolicy {
    /// Trailing bars the indicator may rewrite. Zero means every published
    /// value is final the moment its bar closes.
    pub revision_window_bars: usize,
    /// Leading bars whose values are ignored entirely, for indicators that
    /// publish placeholder output before their window fills.
    pub warmup_bars: usize,
    /// Absolute difference below which two finite values are the same. Exists
    /// for accumulator drift, not to hide small repaints — keep it near the
    /// float noise floor.
    pub tolerance: f64,
    /// Findings retained. The report says when it truncated rather than
    /// pretending the extra findings did not exist.
    pub max_findings: usize,
}

impl Default for RepaintPolicy {
    fn default() -> Self {
        Self {
            revision_window_bars: 0,
            warmup_bars: 0,
            tolerance: 0.0,
            max_findings: 256,
        }
    }
}

/// One value that changed after its bar had closed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepaintFinding {
    /// Index into the indicator's declared output names.
    pub output_index: usize,
    pub output_name: String,
    /// The bar whose published value moved.
    pub bar_index: usize,
    /// The bar count the indicator had been given when the value moved, i.e.
    /// the event responsible.
    pub observed_after_bars: usize,
    /// How far back the mutated bar was at that moment. Always greater than
    /// the declared revision window, or it would not be a finding.
    pub bars_back: usize,
    pub previous_value: f64,
    pub new_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepaintReport {
    pub schema_version: u32,
    pub policy: RepaintPolicy,
    pub bars_scanned: usize,
    pub outputs_scanned: usize,
    /// Ordered by (bar_index, output_index, observed_after_bars) so two runs
    /// over the same input produce byte-identical reports.
    pub findings: Vec<RepaintFinding>,
    /// Findings that occurred but were not retained.
    pub findings_omitted: usize,
}

impl RepaintReport {
    /// True when no closed-bar value ever moved outside the declared window.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty() && self.findings_omitted == 0
    }

    /// The first bar at which this indicator repainted, if any.
    pub fn first_repainted_bar(&self) -> Option<usize> {
        self.findings.first().map(|finding| finding.bar_index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepaintError {
    TooManyBars {
        limit: usize,
        found: usize,
    },
    TooManyOutputs {
        limit: usize,
        found: usize,
    },
    InvalidPolicy,
    /// The indicator returned a series whose shape does not match the bar
    /// prefix it was given, so nothing can be compared meaningfully.
    ShapeMismatch {
        output_index: usize,
        expected: usize,
        found: usize,
    },
    /// The indicator changed how many outputs it publishes partway through.
    OutputCountChanged {
        expected: usize,
        found: usize,
    },
}

impl std::fmt::Display for RepaintError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "repaint diagnostic failed: {self:?}")
    }
}

impl std::error::Error for RepaintError {}

/// An indicator this diagnostic can interrogate.
///
/// The contract is deliberately "given these bars, produce the whole series"
/// rather than "step one bar": an indicator that cannot be asked what it
/// *would have shown* after `n` bars cannot be checked for repainting at all,
/// and one that only exposes its latest value hides exactly the mutation this
/// looks for.
pub trait ObservableIndicator {
    /// Stable names, one per output series. Must not change between calls.
    fn output_names(&self) -> Vec<String>;

    /// The complete output series over `bars`, as `[output][bar]`. Every inner
    /// series must have exactly `bars.len()` entries; use a non-finite value
    /// for bars where the indicator has nothing to say.
    fn evaluate(&mut self, bars: &[SimBar]) -> Vec<Vec<f64>>;
}

/// Two published values are the same when both are undefined, or both finite
/// and within tolerance. A finite value becoming undefined — or the reverse —
/// is a change: a level that vanishes repaints just as surely as one that moves.
fn unchanged(previous: f64, current: f64, tolerance: f64) -> bool {
    match (previous.is_finite(), current.is_finite()) {
        (false, false) => true,
        (true, true) => (previous - current).abs() <= tolerance,
        _ => false,
    }
}

/// Runs the §11.5 repainting test over `bars`.
///
/// Evaluates the indicator once per prefix and compares every closed-bar value
/// against what the previous prefix published for the same bar.
pub fn diagnose_repainting(
    indicator: &mut dyn ObservableIndicator,
    bars: &[SimBar],
    policy: RepaintPolicy,
) -> Result<RepaintReport, RepaintError> {
    if bars.len() > MAX_REPAINT_BARS {
        return Err(RepaintError::TooManyBars {
            limit: MAX_REPAINT_BARS,
            found: bars.len(),
        });
    }
    if !policy.tolerance.is_finite() || policy.tolerance < 0.0 {
        return Err(RepaintError::InvalidPolicy);
    }

    let names = indicator.output_names();
    if names.len() > MAX_REPAINT_OUTPUTS {
        return Err(RepaintError::TooManyOutputs {
            limit: MAX_REPAINT_OUTPUTS,
            found: names.len(),
        });
    }

    let mut findings: Vec<RepaintFinding> = Vec::new();
    let mut findings_omitted = 0_usize;
    let mut previous: Option<Vec<Vec<f64>>> = None;

    for prefix_len in 1..=bars.len() {
        let series = indicator.evaluate(&bars[..prefix_len]);
        if series.len() != names.len() {
            return Err(RepaintError::OutputCountChanged {
                expected: names.len(),
                found: series.len(),
            });
        }
        for (output_index, values) in series.iter().enumerate() {
            if values.len() != prefix_len {
                return Err(RepaintError::ShapeMismatch {
                    output_index,
                    expected: prefix_len,
                    found: values.len(),
                });
            }
        }

        if let Some(before) = &previous {
            // Only bars that had already closed *and* sat outside the declared
            // revision window when this evaluation ran can produce a finding.
            let newest_comparable = prefix_len - 1;
            for (output_index, values) in series.iter().enumerate() {
                for bar_index in policy.warmup_bars..newest_comparable {
                    let bars_back = newest_comparable - bar_index;
                    if bars_back <= policy.revision_window_bars {
                        continue;
                    }
                    let was = before[output_index][bar_index];
                    let now = values[bar_index];
                    if unchanged(was, now, policy.tolerance) {
                        continue;
                    }
                    if findings.len() >= policy.max_findings {
                        findings_omitted = findings_omitted.saturating_add(1);
                        continue;
                    }
                    findings.push(RepaintFinding {
                        output_index,
                        output_name: names[output_index].clone(),
                        bar_index,
                        observed_after_bars: prefix_len,
                        bars_back,
                        previous_value: was,
                        new_value: now,
                    });
                }
            }
        }
        previous = Some(series);
    }

    // Deterministic order: earliest mutated bar first, then output, then the
    // event that mutated it. Two runs over the same input must serialize
    // identically.
    findings.sort_by_key(|finding| {
        (
            finding.bar_index,
            finding.output_index,
            finding.observed_after_bars,
        )
    });

    Ok(RepaintReport {
        schema_version: REPAINT_REPORT_SCHEMA_VERSION,
        policy,
        bars_scanned: bars.len(),
        outputs_scanned: names.len(),
        findings,
        findings_omitted,
    })
}

#[cfg(test)]
mod tests;
