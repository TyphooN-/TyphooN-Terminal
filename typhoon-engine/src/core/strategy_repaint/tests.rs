// ── Repainting diagnostic corpus (ADR-135 §13 M2 gate, §11.5) ──────
//
// The gate clause is that "a synthetic repainting indicator is identified at
// the exact mutated bar/output". These tests build indicators whose repainting
// behaviour is known by construction and assert the exact bar, output, and both
// values — not merely that something was flagged.

use super::{ObservableIndicator, RepaintError, RepaintPolicy, diagnose_repainting};
use crate::core::strategy_simulator::SimBar;

const MINUTE_NS: i64 = 60_000_000_000;

fn bars(closes: &[f64]) -> Vec<SimBar> {
    closes
        .iter()
        .enumerate()
        .map(|(index, close)| SimBar {
            open_time_ns: index as i64 * MINUTE_NS,
            close_time_ns: index as i64 * MINUTE_NS + MINUTE_NS - 1,
            open: *close,
            high: *close + 1.0,
            low: *close - 1.0,
            close: *close,
            volume: 1_000.0,
        })
        .collect()
}

/// A trailing simple moving average. Bar `i`'s value depends only on bars
/// `i - period + 1 ..= i`, so it can never change once published.
struct TrailingSma {
    period: usize,
}

impl ObservableIndicator for TrailingSma {
    fn output_names(&self) -> Vec<String> {
        vec!["sma".to_string()]
    }

    fn evaluate(&mut self, bars: &[SimBar]) -> Vec<Vec<f64>> {
        let values = (0..bars.len())
            .map(|index| {
                if index + 1 < self.period {
                    return f64::NAN;
                }
                let window = &bars[index + 1 - self.period..=index];
                window.iter().map(|bar| bar.close).sum::<f64>() / self.period as f64
            })
            .collect();
        vec![values]
    }
}

/// A centred moving average: bar `i` averages `i - half ..= i + half`, so every
/// value stays provisional until `half` more bars have arrived. This is the
/// classic repainting shape — it looks smooth and predictive on a finished
/// chart and is untradeable in real time.
struct CentredSma {
    half: usize,
}

impl ObservableIndicator for CentredSma {
    fn output_names(&self) -> Vec<String> {
        vec!["centred".to_string()]
    }

    fn evaluate(&mut self, bars: &[SimBar]) -> Vec<Vec<f64>> {
        let values = (0..bars.len())
            .map(|index| {
                let start = index.saturating_sub(self.half);
                let end = (index + self.half).min(bars.len() - 1);
                let window = &bars[start..=end];
                window.iter().map(|bar| bar.close).sum::<f64>() / window.len() as f64
            })
            .collect();
        vec![values]
    }
}

/// Publishes the running maximum close, but rewrites bar 2's value the moment a
/// fifth bar exists. A single, exactly-placed mutation, so the diagnostic's
/// precision can be asserted rather than inferred.
struct SingleMutation;

impl ObservableIndicator for SingleMutation {
    fn output_names(&self) -> Vec<String> {
        vec!["level".to_string(), "untouched".to_string()]
    }

    fn evaluate(&mut self, bars: &[SimBar]) -> Vec<Vec<f64>> {
        let mut level: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
        let untouched: Vec<f64> = bars.iter().map(|bar| bar.high).collect();
        if bars.len() >= 5 {
            level[2] = 999.0;
        }
        vec![level, untouched]
    }
}

#[test]
fn a_trailing_indicator_is_clean() {
    let mut indicator = TrailingSma { period: 3 };
    let report = diagnose_repainting(
        &mut indicator,
        &bars(&[10.0, 11.0, 12.0, 13.0, 14.0, 15.0]),
        RepaintPolicy {
            warmup_bars: 2,
            ..RepaintPolicy::default()
        },
    )
    .expect("diagnoses");

    assert!(report.is_clean(), "a trailing SMA cannot repaint");
    assert_eq!(report.first_repainted_bar(), None);
    assert_eq!(report.bars_scanned, 6);
    assert_eq!(report.outputs_scanned, 1);
}

/// The gate clause: the exact bar and output are named, with both values.
#[test]
fn a_synthetic_repaint_is_identified_at_the_exact_bar_and_output() {
    let closes = [10.0, 11.0, 12.0, 13.0, 14.0];
    let mut indicator = SingleMutation;
    let report = diagnose_repainting(&mut indicator, &bars(&closes), RepaintPolicy::default())
        .expect("diagnoses");

    assert_eq!(report.findings.len(), 1, "exactly one value was rewritten");
    let finding = &report.findings[0];
    assert_eq!(finding.bar_index, 2, "bar 2 is the one that moved");
    assert_eq!(finding.output_index, 0);
    assert_eq!(finding.output_name, "level");
    assert_eq!(
        finding.observed_after_bars, 5,
        "the fifth bar is the event that rewrote it"
    );
    assert_eq!(finding.bars_back, 2, "bar 2 was two bars back by then");
    assert_eq!(finding.previous_value, 12.0, "it had published the close");
    assert_eq!(finding.new_value, 999.0, "and then published this instead");
    assert!(!report.is_clean());
    assert_eq!(report.first_repainted_bar(), Some(2));
}

#[test]
fn a_centred_average_repaints_every_bar_inside_its_look_ahead() {
    let mut indicator = CentredSma { half: 2 };
    let report = diagnose_repainting(
        &mut indicator,
        &bars(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0]),
        RepaintPolicy::default(),
    )
    .expect("diagnoses");

    assert!(!report.is_clean(), "a centred average must be caught");
    // Bar `i` averages `i-2 ..= min(i+2, n-1)` over a prefix of `n` bars, so its
    // value keeps moving until `n >= i + 3`. It is only *compared* once it is no
    // longer the newest bar, i.e. from `n >= i + 2`. Both hold for
    // `i + 2 <= n <= i + 3`, so over seven bars every bar from 0 to 5 is caught;
    // bar 6 is never anything but the newest, so nothing ever contradicts it.
    let mutated: Vec<usize> = {
        let mut seen: Vec<usize> = report
            .findings
            .iter()
            .map(|finding| finding.bar_index)
            .collect();
        seen.dedup();
        seen
    };
    assert_eq!(mutated, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(report.first_repainted_bar(), Some(0));
}

/// A declared revision window is the difference between honest provisional
/// output and a silent repaint. The same centred average is clean once its
/// two-bar look-ahead is declared.
#[test]
fn a_declared_revision_window_makes_provisional_output_honest() {
    let series = bars(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0]);
    let mut indicator = CentredSma { half: 2 };
    let declared = diagnose_repainting(
        &mut indicator,
        &series,
        RepaintPolicy {
            revision_window_bars: 2,
            ..RepaintPolicy::default()
        },
    )
    .expect("diagnoses");
    assert!(
        declared.is_clean(),
        "revisions inside a declared window are expected behaviour"
    );

    // One bar short of the truth is still a finding: the window has to cover
    // the real look-ahead, not merely gesture at it.
    let mut indicator = CentredSma { half: 2 };
    let understated = diagnose_repainting(
        &mut indicator,
        &series,
        RepaintPolicy {
            revision_window_bars: 1,
            ..RepaintPolicy::default()
        },
    )
    .expect("diagnoses");
    assert!(!understated.is_clean(), "an understated window is caught");
}

#[test]
fn a_value_that_disappears_counts_as_a_repaint() {
    /// Publishes a level and then withdraws bar 1's once four bars exist.
    struct Vanishing;
    impl ObservableIndicator for Vanishing {
        fn output_names(&self) -> Vec<String> {
            vec!["level".to_string()]
        }
        fn evaluate(&mut self, bars: &[SimBar]) -> Vec<Vec<f64>> {
            let mut values: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
            if bars.len() >= 4 {
                values[1] = f64::NAN;
            }
            vec![values]
        }
    }

    let mut indicator = Vanishing;
    let report = diagnose_repainting(
        &mut indicator,
        &bars(&[1.0, 2.0, 3.0, 4.0]),
        RepaintPolicy::default(),
    )
    .expect("diagnoses");

    assert_eq!(report.findings.len(), 1);
    let finding = &report.findings[0];
    assert_eq!(finding.bar_index, 1);
    assert_eq!(finding.previous_value, 2.0);
    assert!(
        finding.new_value.is_nan(),
        "a level that vanishes repaints as surely as one that moves"
    );
}

#[test]
fn warmup_bars_are_excluded_from_the_scan() {
    /// Rewrites bar 0 — inside any warmup — and bar 3, outside it.
    struct WarmupThenReal;
    impl ObservableIndicator for WarmupThenReal {
        fn output_names(&self) -> Vec<String> {
            vec!["level".to_string()]
        }
        fn evaluate(&mut self, bars: &[SimBar]) -> Vec<Vec<f64>> {
            let mut values: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
            if bars.len() >= 6 {
                values[0] = -1.0;
                values[3] = -1.0;
            }
            vec![values]
        }
    }

    let series = bars(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let mut indicator = WarmupThenReal;
    let report = diagnose_repainting(
        &mut indicator,
        &series,
        RepaintPolicy {
            warmup_bars: 2,
            ..RepaintPolicy::default()
        },
    )
    .expect("diagnoses");

    let mutated: Vec<usize> = report
        .findings
        .iter()
        .map(|finding| finding.bar_index)
        .collect();
    assert_eq!(mutated, vec![3], "bar 0 is inside warmup and not scanned");
}

#[test]
fn the_report_is_deterministic_and_round_trips() {
    let series = bars(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
    let run = || {
        let mut indicator = CentredSma { half: 1 };
        diagnose_repainting(&mut indicator, &series, RepaintPolicy::default()).expect("diagnoses")
    };
    let first = run();
    let second = run();
    assert_eq!(first, second, "the same input must produce the same report");
    assert_eq!(
        serde_json::to_string(&first).expect("serializes"),
        serde_json::to_string(&second).expect("serializes"),
        "and serialize identically"
    );

    let json = serde_json::to_vec(&first).expect("serializes");
    let restored: super::RepaintReport = serde_json::from_slice(&json).expect("round trips");
    assert_eq!(restored, first);

    // Findings are ordered by the bar that moved, so a reader sees the earliest
    // damage first rather than whichever prefix happened to notice it.
    let ordered: Vec<usize> = first
        .findings
        .iter()
        .map(|finding| finding.bar_index)
        .collect();
    let mut sorted = ordered.clone();
    sorted.sort_unstable();
    assert_eq!(ordered, sorted);
}

#[test]
fn findings_are_capped_and_the_omission_is_reported() {
    let series = bars(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0]);
    let mut indicator = CentredSma { half: 3 };
    let report = diagnose_repainting(
        &mut indicator,
        &series,
        RepaintPolicy {
            max_findings: 2,
            ..RepaintPolicy::default()
        },
    )
    .expect("diagnoses");

    assert_eq!(report.findings.len(), 2, "capped");
    assert!(
        report.findings_omitted > 0,
        "the cap must be reported, not hidden"
    );
    assert!(!report.is_clean(), "a truncated report is still not clean");
}

#[test]
fn a_misshapen_indicator_is_rejected_rather_than_compared() {
    /// Returns a series shorter than the bars it was given.
    struct WrongLength;
    impl ObservableIndicator for WrongLength {
        fn output_names(&self) -> Vec<String> {
            vec!["level".to_string()]
        }
        fn evaluate(&mut self, _bars: &[SimBar]) -> Vec<Vec<f64>> {
            vec![vec![1.0]]
        }
    }

    let mut indicator = WrongLength;
    assert_eq!(
        diagnose_repainting(&mut indicator, &bars(&[1.0, 2.0]), RepaintPolicy::default()),
        Err(RepaintError::ShapeMismatch {
            output_index: 0,
            expected: 2,
            found: 1,
        })
    );

    /// Publishes a second output only once it has enough bars.
    struct GrowingOutputs;
    impl ObservableIndicator for GrowingOutputs {
        fn output_names(&self) -> Vec<String> {
            vec!["a".to_string()]
        }
        fn evaluate(&mut self, bars: &[SimBar]) -> Vec<Vec<f64>> {
            let series: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
            if bars.len() >= 2 {
                vec![series.clone(), series]
            } else {
                vec![series]
            }
        }
    }

    let mut indicator = GrowingOutputs;
    assert_eq!(
        diagnose_repainting(&mut indicator, &bars(&[1.0, 2.0]), RepaintPolicy::default()),
        Err(RepaintError::OutputCountChanged {
            expected: 1,
            found: 2,
        })
    );
}

#[test]
fn the_scan_is_bounded_before_it_runs() {
    let mut indicator = TrailingSma { period: 3 };
    let oversized = vec![
        SimBar {
            open_time_ns: 0,
            close_time_ns: 1,
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 0.0,
        };
        super::MAX_REPAINT_BARS + 1
    ];
    assert_eq!(
        diagnose_repainting(&mut indicator, &oversized, RepaintPolicy::default()),
        Err(RepaintError::TooManyBars {
            limit: super::MAX_REPAINT_BARS,
            found: super::MAX_REPAINT_BARS + 1,
        })
    );

    assert_eq!(
        diagnose_repainting(
            &mut indicator,
            &bars(&[1.0]),
            RepaintPolicy {
                tolerance: f64::NAN,
                ..RepaintPolicy::default()
            }
        ),
        Err(RepaintError::InvalidPolicy)
    );
}
