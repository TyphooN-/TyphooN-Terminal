use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ChartSplit {
    pub ex_ts_ms: i64,
    pub pre_split_factor: f64,
}

/// Back-adjust an unadjusted source's buckets for known splits: each bar before a
/// split's ex-date is scaled by the cumulative product of all later splits' factors.
/// Exact and source-independent — unlike the cross-source era inference, it works
/// even when no adjusted reference (Alpaca) is present and across a single split era.
fn chart_back_adjust_bars_for_splits(
    bucketed: &mut std::collections::BTreeMap<i64, Bar>,
    splits: &[ChartSplit],
) {
    if splits.is_empty() {
        return;
    }
    for (ts, bar) in bucketed.iter_mut() {
        let mut factor = 1.0;
        for s in splits {
            if *ts < s.ex_ts_ms {
                factor *= s.pre_split_factor;
            }
        }
        if (factor - 1.0).abs() > 1e-9 {
            bar.open *= factor;
            bar.high *= factor;
            bar.low *= factor;
            bar.close *= factor;
        }
    }
}

/// Median of the positive closes in `[lo, hi)` of a bucket map; `None` if empty.
fn chart_median_close(
    bucketed: &std::collections::BTreeMap<i64, Bar>,
    lo: i64,
    hi: i64,
) -> Option<f64> {
    let mut v: Vec<f64> = bucketed
        .range(lo..hi)
        .map(|(_, b)| b.close)
        .filter(|c| *c > 0.0)
        .collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(v[v.len() / 2])
}

/// Back-adjust a RAW *trusted* source (Alpaca, normally
/// `adjustment=all`) that nonetheless served UNADJUSTED bars across a fresh
/// reverse split — the HUBC 1-for-20 case (2026-06): Alpaca carried a raw ~20×
/// cliff, there was no kraken-equities source to anchor the dated back-adjust
/// above, and a lone reverse-split era is below the depth-inference's ≥2-era
/// bar, so the 20× discontinuity painted vs TradingView.
///
/// Distinct from the date-exact kraken-equities path, this fires ONLY when the
/// source itself shows a reverse-split-shaped scale step near a known split, so
/// an already-adjusted (continuous) source has no step and is left untouched —
/// it can never be double-adjusted. Two robustness choices matter:
///   * the cut snaps to the source's *actual* step bucket (Alpaca's lands ~2
///     sessions before the published ex-date — HUBC stepped 06-04 vs 06-08), and
///   * the lift uses the PUBLISHED factor, not the realized step ratio, so a
///     concurrent split-day price move (HUBC fell ~68% that day) can't distort it.
fn chart_back_adjust_raw_trusted_source_for_splits(
    bucketed: &mut std::collections::BTreeMap<i64, Bar>,
    splits: &[ChartSplit],
) {
    const DAY_MS: i64 = 86_400_000;
    const PRE_SLACK_MS: i64 = 14 * DAY_MS; // search back this far for the step
    const POST_SLACK_MS: i64 = 7 * DAY_MS; // ...and a little past the ex-date
    const ERA_MS: i64 = 21 * DAY_MS; // era windows confirming the step
    const MIN_STEP_RATIO: f64 = 2.0; // a reverse split shows ≥2× even after a same-day drop
    // Detect qualifying cuts on the ORIGINAL series first, then apply the
    // cumulative product (order-independent across multiple splits).
    let mut cuts: Vec<(i64, f64)> = Vec::new(); // (boundary_ts, factor)
    for s in splits {
        if s.pre_split_factor <= 1.0 + 1e-9 {
            continue; // reverse splits only; forward/era cases handled elsewhere
        }
        // Prefer a visible single-bar step if present (largest upward close jump).
        let mut prev: Option<f64> = None;
        let mut boundary: Option<(i64, f64)> = None;
        for (ts, bar) in bucketed.range(s.ex_ts_ms - PRE_SLACK_MS..=s.ex_ts_ms + POST_SLACK_MS) {
            if let Some(pc) = prev {
                if pc > 0.0 && bar.close > 0.0 {
                    let r = bar.close / pc;
                    if boundary.map(|(_, br)| r > br).unwrap_or(true) {
                        boundary = Some((*ts, r));
                    }
                }
            }
            prev = Some(bar.close);
        }
        // Era-level confirmation around the known ex-date (handles volatile
        // split days where no single close ratio reaches MIN_STEP_RATIO).
        // Use ex-date itself as boundary when the step is masked by price action.
        let use_ex_boundary = boundary.as_ref().map_or(true, |(_, r)| *r < MIN_STEP_RATIO);
        let boundary_ts = if use_ex_boundary {
            s.ex_ts_ms
        } else {
            boundary.unwrap().0
        };
        match (
            chart_median_close(bucketed, boundary_ts - ERA_MS, boundary_ts),
            chart_median_close(bucketed, boundary_ts, boundary_ts + ERA_MS),
        ) {
            (Some(pre), Some(post)) if pre > 0.0 && post / pre >= s.pre_split_factor.sqrt() => {
                cuts.push((boundary_ts, s.pre_split_factor));
            }
            _ => {}
        }
    }
    if cuts.is_empty() {
        return;
    }
    for (ts, bar) in bucketed.iter_mut() {
        let mut factor = 1.0;
        for &(boundary_ts, f) in &cuts {
            if *ts < boundary_ts {
                factor *= f;
            }
        }
        if (factor - 1.0).abs() > 1e-9 {
            bar.open *= factor;
            bar.high *= factor;
            bar.low *= factor;
            bar.close *= factor;
        }
    }
}

/// Convert a stored FMP `StockSplit` into a `ChartSplit` (parse the ex-date, derive
/// the pre-split multiplier). Skips malformed/zero entries.
fn chart_split_from_stock_split(
    s: &typhoon_engine::core::research::StockSplit,
) -> Option<ChartSplit> {
    if s.numerator <= 0.0 || s.denominator <= 0.0 {
        return None;
    }
    let date = chrono::NaiveDate::parse_from_str(&s.date, "%Y-%m-%d").ok()?;
    let ex_ts_ms = date.and_hms_opt(0, 0, 0)?.and_utc().timestamp_millis();
    Some(ChartSplit {
        ex_ts_ms,
        pre_split_factor: s.denominator / s.numerator,
    })
}

/// Curated corporate actions for symbols where the FMP split feed is missing or
/// unreliable. Free-tier FMP omits many microcap reverse splits, and a node that
/// has never scraped `research_stock_splits` has the table empty (or
/// absent) entirely — which starves the exact back-adjustment
/// ([`chart_back_adjust_bars_for_splits`]) of the split it needs, so raw
/// pre-split history (Kraken xStock bars) gets painted on the wrong scale. That
/// is the WOK reverse-split discontinuity vs TradingView: the merge code is
/// correct and tested, it was simply never handed the split. These entries
/// supplement [`chart_known_splits_from_cache`] so the back-adjust still fires
/// offline / without an FMP key. See ADR-122.
///
/// `pre_split_factor` = old shares / new shares (= 100 for a 1-for-100 reverse
/// split). Dates are the split ex-date at 00:00 UTC. Verify each against the
/// issuer's actual action before adding.
pub(crate) fn chart_curated_known_splits(symbol: &str) -> Vec<ChartSplit> {
    // (symbol, ex-date "YYYY-MM-DD", pre_split_factor = denominator/numerator)
    const CURATED: &[(&str, &str, f64)] = &[
        // WORK Medical Technology Group — 1-for-100 reverse split.
        ("WOK", "2025-12-29", 100.0),
        // HUB Cyber Security — 1-for-20 reverse split. Effective 11:59pm ET
        // 2026-06-05; Nasdaq split-adjusted trading from 2026-06-08 (new CUSIP
        // M6000J192). FMP omitted it and there is no kraken-equities source, so
        // the merge had no split to anchor on: Alpaca's pre-split history is raw
        // (unadjusted) while its post-split bars sit ~20× higher, and the lone
        // reverse-split era is below the depth-inference's ≥2-era bar — so the
        // 20× discontinuity painted vs TradingView's adjusted series.
        ("HUBC", "2026-06-08", 20.0),
    ];
    let su = symbol.trim().to_ascii_uppercase();
    CURATED
        .iter()
        .filter(|(sym, _, _)| su == *sym)
        .filter_map(|(_, date, factor)| {
            let d = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
            Some(ChartSplit {
                ex_ts_ms: d.and_hms_opt(0, 0, 0)?.and_utc().timestamp_millis(),
                pre_split_factor: *factor,
            })
        })
        .collect()
}

/// Load known splits for `symbol` for use in equity-merge back-adjustment:
/// FMP-sourced rows from the research cache (`research_stock_splits`, read-only),
/// supplemented by [`chart_curated_known_splits`] for actions FMP missed.
/// Empty only when neither source knows a split (the era-inference fallback then
/// applies). Curated entries are deduped against cached rows by ex-date so real
/// FMP data takes precedence when both are present.
fn chart_known_splits_from_cache(cache: &SqliteCache, symbol: &str) -> Vec<ChartSplit> {
    let mut splits: Vec<ChartSplit> = Vec::new();
    if let Ok(conn) = cache.read_connection() {
        if let Ok(Some(rows)) = typhoon_engine::core::research::get_stock_splits(&conn, symbol) {
            splits = rows
                .iter()
                .filter_map(chart_split_from_stock_split)
                .collect();
        }
    }
    for c in chart_curated_known_splits(symbol) {
        let dup = splits
            .iter()
            .any(|s| (s.ex_ts_ms - c.ex_ts_ms).abs() < 86_400_000);
        if !dup {
            splits.push(c);
        }
    }
    splits
}

pub(crate) fn chart_merge_equity_raw_bars(
    timeframe: &str,
    sources: &[(&str, &[(i64, f64, f64, f64, f64, f64)])],
    splits: &[ChartSplit],
) -> Vec<Bar> {
    chart_merge_equity_raw_bars_with_primary(
        timeframe,
        sources,
        splits,
        chart_merge_primary_broker(),
    )
}

/// As [`chart_merge_equity_raw_bars`], but with an explicit primary broker (the
/// trusted-scale source). The no-arg wrapper reads the process-wide selection;
/// this variant lets callers/tests pin the orientation. ADR-126.
pub(crate) fn chart_merge_equity_raw_bars_with_primary(
    timeframe: &str,
    sources: &[(&str, &[(i64, f64, f64, f64, f64, f64)])],
    splits: &[ChartSplit],
    primary: OrderBroker,
) -> Vec<Bar> {
    use std::collections::BTreeMap;
    const TRUSTED_MAX_RANK: u8 = 2; // alpaca and better define the price scale

    // Validate + bucket each usable source, tagged by its priority rank.
    let mut tagged: Vec<(u8, BTreeMap<i64, Bar>)> = Vec::new();
    for (source, raw) in sources {
        let Some(rank) = chart_equity_source_rank_for(source, primary) else {
            continue;
        };
        if !chart_source_bars_match_timeframe(source, timeframe, raw) {
            continue;
        }
        let mut bucketed = chart_bucket_valid_source_bars(timeframe, raw);
        // kraken-equities iapi returns RAW (unadjusted) xStock bars. Back-adjust by
        // KNOWN splits at their known ex-dates so pre-split history lands on the
        // post-split scale (matching Alpaca `adjustment=all` + TradingView), instead
        // of relying solely on the cross-source era inference below.
        if *source == "kraken-equities" {
            chart_back_adjust_bars_for_splits(&mut bucketed, splits);
        } else if rank <= TRUSTED_MAX_RANK {
            // Alpaca is normally split-adjusted, but can serve RAW bars
            // across a fresh microcap reverse split (HUBC 1-for-20). Lift their
            // pre-split history only when the bars themselves show the split
            // step — already-adjusted bars are continuous and left untouched.
            chart_back_adjust_raw_trusted_source_for_splits(&mut bucketed, splits);
        }
        if !bucketed.is_empty() {
            tagged.push((rank, bucketed));
        }
    }
    // Best priority first; stable so equal ranks keep input order.
    tagged.sort_by_key(|(rank, _)| *rank);

    // Trusted tier defines the scale: per-bucket, the best rank present wins.
    let mut merged: BTreeMap<i64, Bar> = BTreeMap::new();
    for (rank, bucketed) in &tagged {
        if *rank > TRUSTED_MAX_RANK {
            continue;
        }
        for (bucket, bar) in bucketed {
            merged.entry(*bucket).or_insert_with(|| bar.clone());
        }
    }

    let trusted_merge_is_stale = chart_trusted_equity_merge_is_stale(timeframe, &merged, &tagged);
    if trusted_merge_is_stale {
        merged.clear();
    }

    if merged.is_empty() {
        // No trusted reference — best-effort per-bucket priority over depth.
        for (rank, bucketed) in &tagged {
            if trusted_merge_is_stale && *rank <= TRUSTED_MAX_RANK {
                continue;
            }
            for (bucket, bar) in bucketed {
                merged.entry(*bucket).or_insert_with(|| bar.clone());
            }
        }
        return merged.into_values().collect();
    }

    // Trusted-tier split-adjustment reconciliation. The best-rank trusted source
    // (kraken-equities iapi) returns RAW xStock bars, while Alpaca returns
    // split-adjusted bars (`adjustment=all`). Across a reverse split (WOK 1-for-100,
    // 2025-12) the raw source sits on a different scale and — out-ranking Alpaca
    // per bucket — paints unadjusted pre-split history (the December discontinuity
    // TradingView never shows). Where the raw source diverges from the adjusted
    // reference across a whole consistent ERA (not a single bad print), adopt the
    // adjusted bars so the series stays continuous.
    chart_reconcile_trusted_split_adjustment(&mut merged, &tagged);

    // Independent adjusted-depth reconciliation. Kraken xStock history can be
    // Alpaca-derived, so Kraken + Alpaca may share the same mis-adjusted split
    // history. When Yahoo/TradingView-style data agrees recently but exposes
    // older stable split-era ratios, let it replace those trusted OHLC eras.
    chart_reconcile_depth_split_adjustment(&mut merged, &tagged);

    // Trusted-tier outlier correction. A trusted feed can momentarily emit a
    // bad print — a thin microcap whose provider mis-applies a corporate action
    // (WOK doubled to ~2× on Alpaca for two days in 2026-06 while Yahoo,
    // TradingView and the live tape all stayed flat). The depth tier only fills
    // *gaps*, so a bad trusted bar would otherwise be charted unchallenged and
    // poison the autoscale + every MA/ATR. Where a depth corroborator overlaps on
    // a locally-consistent recent scale, replace any trusted bar that diverges
    // from the rescaled corroborator by more than OUTLIER_RATIO. Deliberately
    // recent-window only: deep history can legitimately sit on a different scale
    // per split era (an unadjusted depth source), so we never "correct" there.
    const OUTLIER_RATIO: f64 = 1.5;
    for (rank, bucketed) in &tagged {
        if *rank <= TRUSTED_MAX_RANK {
            continue;
        }
        let Some((scale, window_start)) = chart_recent_overlap_scale(&merged, bucketed) else {
            continue;
        };
        // Compare close, high, AND low against the rescaled corroborator. A bad
        // trusted print can be a full-candle doubling (close diverges) or a lone
        // wick spike (only the high diverges) — the WOK H4 artifact was the
        // latter, invisible to a close-only check.
        let diverges = |trusted_v: f64, depth_v: f64| -> bool {
            let expected = depth_v * scale;
            expected > 0.0
                && trusted_v > 0.0
                && (trusted_v / expected).max(expected / trusted_v) > OUTLIER_RATIO
        };
        for (bucket, dbar) in bucketed {
            if *bucket < window_start {
                continue; // only adjudicate the recent, locally-consistent window
            }
            let Some(tbar) = merged.get(bucket) else {
                continue;
            };
            if diverges(tbar.close, dbar.close)
                || diverges(tbar.high, dbar.high)
                || diverges(tbar.low, dbar.low)
            {
                merged.insert(
                    *bucket,
                    Bar {
                        ts_ms: tbar.ts_ms,
                        open: dbar.open * scale,
                        high: dbar.high * scale,
                        low: dbar.low * scale,
                        close: dbar.close * scale,
                        volume: tbar.volume,
                    },
                );
            }
        }
        break; // only the best valid corroborator adjudicates
    }

    // Splice depth sources in (best rank first), back-adjusted to the trusted
    // scale, filling only buckets not already covered (older history + gaps).
    for (rank, bucketed) in &tagged {
        if *rank <= TRUSTED_MAX_RANK {
            continue;
        }
        let Some(factor) = chart_depth_source_scale_factor(&merged, bucketed) else {
            continue; // unreconcilable scale (unadjusted action) → drop source
        };
        let rescale = (factor - 1.0).abs() > 1e-9;
        for (bucket, bar) in bucketed {
            if merged.contains_key(bucket) {
                continue;
            }
            let bar = if rescale {
                Bar {
                    ts_ms: bar.ts_ms,
                    open: bar.open * factor,
                    high: bar.high * factor,
                    low: bar.low * factor,
                    close: bar.close * factor,
                    volume: bar.volume,
                }
            } else {
                bar.clone()
            };
            merged.insert(*bucket, bar);
        }
    }

    merged.into_values().collect()
}

/// Reconcile a raw best-rank trusted source against a split-adjusted lower-rank
/// trusted source (Alpaca, `adjustment=all`) — see the call site. Only an
/// ERA-WIDE, internally-consistent divergence (a corporate-action scale step, not
/// a single bad print) is overridden, so a lone bad Alpaca bar can't hijack a good
/// raw bar. Buckets in the recent window are left to the Yahoo outlier guard.
fn chart_reconcile_trusted_split_adjustment(
    merged: &mut std::collections::BTreeMap<i64, Bar>,
    tagged: &[(u8, std::collections::BTreeMap<i64, Bar>)],
) {
    const TRUSTED_MAX_RANK: u8 = 2;
    const DIVERGE_RATIO: f64 = 1.5; // a scale step, not noise
    const MIN_ERA: usize = 5; // need a run of divergent buckets, not one bad bar
    const ERA_TOL: f64 = 1.25; // the divergent ratios must share one scale factor

    // The best-rank trusted source is the one that populated `merged`.
    let Some(best_rank) = tagged
        .iter()
        .map(|(rank, _)| *rank)
        .filter(|rank| *rank <= TRUSTED_MAX_RANK)
        .min()
    else {
        return;
    };

    for (rank, adj) in tagged {
        if *rank <= best_rank || *rank > TRUSTED_MAX_RANK {
            continue; // only a lower-rank trusted source is a candidate reference
        }
        // Recent consensus ratio between merged (raw best) and the adjusted
        // reference. They must agree recently (post-split) for the comparison to
        // mean anything; a window straddling the split is rejected by the
        // tightness check inside chart_recent_overlap_scale.
        let Some((consensus, window_start)) = chart_recent_overlap_scale(merged, adj) else {
            continue;
        };
        // Older buckets where merged diverges from the adjusted reference beyond
        // DIVERGE_RATIO of that recent consensus.
        let mut divergent: Vec<(i64, f64)> = Vec::new();
        for (bucket, abar) in adj {
            if *bucket >= window_start {
                continue; // recent window is handled by the outlier guard
            }
            let Some(mbar) = merged.get(bucket) else {
                continue;
            };
            if abar.close <= 0.0 || mbar.close <= 0.0 {
                continue;
            }
            let ratio = mbar.close / abar.close;
            if (ratio / consensus).max(consensus / ratio) > DIVERGE_RATIO {
                divergent.push((*bucket, ratio));
            }
        }
        if divergent.len() < MIN_ERA {
            continue; // not era-wide → could be a single bad print; leave it alone
        }
        // The divergent ratios must be one consistent scale (a split factor), not
        // scattered single-bar errors.
        let mut ratios: Vec<f64> = divergent.iter().map(|(_, r)| *r).collect();
        ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p25 = ratios[ratios.len() / 4];
        let p75 = ratios[ratios.len() * 3 / 4];
        if p25 <= 0.0 || p75 / p25 > ERA_TOL {
            continue;
        }
        // A consistent mis-adjusted era → the adjusted reference is authoritative.
        for (bucket, _) in &divergent {
            if let Some(abar) = adj.get(bucket) {
                merged.insert(*bucket, abar.clone());
            }
        }
    }
}

fn chart_reconcile_depth_split_adjustment(
    merged: &mut std::collections::BTreeMap<i64, Bar>,
    tagged: &[(u8, std::collections::BTreeMap<i64, Bar>)],
) {
    const TRUSTED_MAX_RANK: u8 = 2;
    const DIVERGE_RATIO: f64 = 1.5;
    const ERA_TOL: f64 = 1.25;
    const MIN_ERA: usize = 5;

    for (rank, depth) in tagged {
        if *rank <= TRUSTED_MAX_RANK {
            continue;
        }
        let Some((consensus, window_start)) = chart_recent_overlap_scale(merged, depth) else {
            continue;
        };
        let mut run: Vec<(i64, f64)> = Vec::new();
        let mut runs: Vec<Vec<(i64, f64)>> = Vec::new();
        for (bucket, dbar) in depth {
            if *bucket >= window_start {
                break;
            }
            let Some(tbar) = merged.get(bucket) else {
                chart_stage_depth_split_adjustment_run(&mut runs, &run, ERA_TOL, MIN_ERA);
                run.clear();
                continue;
            };
            if tbar.close <= 0.0 || dbar.close <= 0.0 {
                chart_stage_depth_split_adjustment_run(&mut runs, &run, ERA_TOL, MIN_ERA);
                run.clear();
                continue;
            }
            let ratio = tbar.close / dbar.close;
            if (ratio / consensus).max(consensus / ratio) <= DIVERGE_RATIO {
                chart_stage_depth_split_adjustment_run(&mut runs, &run, ERA_TOL, MIN_ERA);
                run.clear();
                continue;
            }
            let same_era = run
                .last()
                .map(|(_, prev_ratio)| {
                    let lo = ratio.min(*prev_ratio);
                    let hi = ratio.max(*prev_ratio);
                    lo > 0.0 && hi / lo <= ERA_TOL
                })
                .unwrap_or(true);
            if !same_era {
                chart_stage_depth_split_adjustment_run(&mut runs, &run, ERA_TOL, MIN_ERA);
                run.clear();
            }
            run.push((*bucket, ratio));
        }
        chart_stage_depth_split_adjustment_run(&mut runs, &run, ERA_TOL, MIN_ERA);
        // Require at least two older stable divergent eras before promoting a
        // depth source over trusted history. A single old depth-only scale jump
        // can be an unadjusted/bad provider region; two clean eras plus recent
        // agreement matches the WOK/TradingView multi-reverse-split shape.
        //
        // ...AND only when promoting does not let the depth source REDEFINE the
        // price scale. The trusted tier defines the scale (ADR-113); depth may
        // smooth a mis-adjusted continuity but must not relocate bars by orders of
        // magnitude. WOK did two 1-for-100 reverse splits with no kraken-equities
        // source: Alpaca is raw (compact, with split cliffs) while Yahoo is
        // back-adjusted across BOTH splits and so explodes to ~10,000× the recent
        // price in deep history. Promoting that pasted Yahoo's ~36,000 bars over
        // Alpaca's compact ones — the H1/H4 spikes, and an inconsistency vs the
        // (compact) D1/W1 views. The guard keeps depth promotion on the trusted
        // scale, so an exploded-scale depth source is refused and the series stays
        // compact and identical across timeframes.
        if runs.len() >= 1 && chart_depth_promotion_keeps_trusted_scale(depth, &runs, window_start)
        {
            for run in &runs {
                chart_apply_depth_split_adjustment_run(merged, depth, run, consensus);
            }
        }
        break;
    }
}

fn chart_stage_depth_split_adjustment_run(
    runs: &mut Vec<Vec<(i64, f64)>>,
    run: &[(i64, f64)],
    era_tol: f64,
    min_era: usize,
) {
    if run.len() < min_era {
        return;
    }
    let mut ratios: Vec<f64> = run.iter().map(|(_, ratio)| *ratio).collect();
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p25 = ratios[ratios.len() / 4];
    let p75 = ratios[ratios.len() * 3 / 4];
    if p25 <= 0.0 || p75 / p25 > era_tol {
        return;
    }
    runs.push(run.to_vec());
}

fn chart_apply_depth_split_adjustment_run(
    merged: &mut std::collections::BTreeMap<i64, Bar>,
    depth: &std::collections::BTreeMap<i64, Bar>,
    run: &[(i64, f64)],
    consensus: f64,
) {
    if consensus <= 0.0 {
        return;
    }
    for (bucket, _) in run {
        let (Some(depth_bar), Some(trusted_bar)) = (depth.get(bucket), merged.get(bucket)) else {
            continue;
        };
        merged.insert(
            *bucket,
            Bar {
                ts_ms: trusted_bar.ts_ms,
                open: depth_bar.open * consensus,
                high: depth_bar.high * consensus,
                low: depth_bar.low * consensus,
                close: depth_bar.close * consensus,
                volume: trusted_bar.volume,
            },
        );
    }
}

/// Guard for the depth-era promotion: refuse it when adopting the depth source
/// would let it REDEFINE the price scale rather than merely smooth a continuity.
///
/// The trusted tier defines the scale (ADR-113). The "≥2 stable divergent eras"
/// signal is symmetric — it looks the same whether trusted is raw across reverse
/// splits (and depth is the adjusted reference) or depth is back-adjusted onto a
/// runaway scale (and trusted is the compact, real-price one). WOK is the latter:
/// two 1-for-100 reverse splits, no kraken-equities source, so Yahoo's depth
/// history is back-adjusted ×10,000 into the tens of thousands while Alpaca stays
/// on the compact traded scale. Promoting Yahoo there pastes ~36,000-priced bars
/// over the chart (the H1/H4 spikes).
///
/// We keep promotion only while the depth source stays within `SCALE_CAP` of its
/// own recent (consensus-window) level across every divergent era — i.e. depth is
/// correcting a few-fold mis-adjustment, not relocating the series by orders of
/// magnitude. A genuinely adjusted depth reference (compact multi-split history)
/// passes; a runaway back-adjusted one (WOK/Yahoo) is refused and the merge keeps
/// the trusted scale, identical across every timeframe.
fn chart_depth_promotion_keeps_trusted_scale(
    depth: &std::collections::BTreeMap<i64, Bar>,
    runs: &[Vec<(i64, f64)>],
    window_start: i64,
) -> bool {
    // A single common reverse split is ~10×; two stacked are ~100×. Beyond this a
    // divergent era is a back-adjustment runaway, not a real multi-era price range.
    const SCALE_CAP: f64 = 50.0;

    let median = |mut v: Vec<f64>| -> Option<f64> {
        v.retain(|c| *c > 0.0 && c.is_finite());
        if v.is_empty() {
            return None;
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(v[v.len() / 2])
    };
    // The scale the corrected series would sit on: depth's own recent level.
    let Some(recent) = median(
        depth
            .range(window_start..)
            .map(|(_, bar)| bar.close)
            .collect(),
    ) else {
        return false;
    };
    runs.iter().all(|run| {
        match median(
            run.iter()
                .filter_map(|(b, _)| depth.get(b))
                .map(|bar| bar.close)
                .collect(),
        ) {
            Some(era) => (era / recent).max(recent / era) <= SCALE_CAP,
            None => true,
        }
    })
}

fn chart_trusted_equity_merge_is_stale(
    timeframe: &str,
    trusted: &std::collections::BTreeMap<i64, Bar>,
    tagged: &[(u8, std::collections::BTreeMap<i64, Bar>)],
) -> bool {
    let Some(trusted_last) = trusted.keys().next_back().copied() else {
        return false;
    };
    let Some(depth_last) = tagged
        .iter()
        .filter(|(rank, _)| *rank > 2)
        .filter_map(|(_, bucketed)| bucketed.keys().next_back().copied())
        .max()
    else {
        return false;
    };
    depth_last.saturating_sub(trusted_last) > chart_stale_trusted_equity_gap_ms(timeframe)
}

fn chart_stale_trusted_equity_gap_ms(timeframe: &str) -> i64 {
    let hour = 3_600_000i64;
    let day = 24 * hour;
    match timeframe {
        "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" => 10 * day,
        "1Day" => 45 * day,
        "1Week" => 120 * day,
        "1Month" => 370 * day,
        _ => 45 * day,
    }
}

/// Validate, bucket, and de-duplicate one raw provider series into
/// `bucket → Bar` (the latest tick within a bucket wins). Bars that fail basic
/// sanity (non-positive / non-finite OHLC, high < low, non-positive ts) drop.
fn chart_bucket_valid_source_bars(
    timeframe: &str,
    raw: &[(i64, f64, f64, f64, f64, f64)],
) -> std::collections::BTreeMap<i64, Bar> {
    let mut out: std::collections::BTreeMap<i64, Bar> = std::collections::BTreeMap::new();
    for (ts, o, h, l, c, v) in raw.iter().copied() {
        if ts <= 0
            || o <= 0.0
            || h <= 0.0
            || l <= 0.0
            || c <= 0.0
            || !o.is_finite()
            || !h.is_finite()
            || !l.is_finite()
            || !c.is_finite()
            || h < l
        {
            continue;
        }
        let bucket = chart_merge_bucket_ts(timeframe, ts);
        let bar = Bar {
            ts_ms: ts,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: v,
        };
        match out.get(&bucket) {
            Some(existing) if existing.ts_ms > ts => {}
            _ => {
                out.insert(bucket, bar);
            }
        }
    }
    out
}

/// Back-adjustment factor that brings a depth source onto the trusted scale:
/// `median(trusted_close / depth_close)` over the buckets they share. Returns
/// `None` — meaning "drop this source" — when there is no overlap, or when the
/// overlap is large enough to judge yet the per-bucket ratios span more than
/// `SCALE_TOL` (p90/p10). A continuously-offset source (a clean, unadjusted but
/// constant split) has a near-constant ratio and is kept & rescaled; an
/// internally-inconsistent source (an unadjusted action mid-history, like
/// Yahoo's WOK) trips the tolerance and is rejected.
fn chart_depth_source_scale_factor(
    trusted: &std::collections::BTreeMap<i64, Bar>,
    depth: &std::collections::BTreeMap<i64, Bar>,
) -> Option<f64> {
    const CONSISTENCY_MIN_OVERLAP: usize = 8;
    const SCALE_TOL: f64 = 3.0;

    let mut factors: Vec<f64> = depth
        .iter()
        .filter_map(|(bucket, dbar)| {
            trusted
                .get(bucket)
                .filter(|tbar| tbar.close > 0.0 && dbar.close > 0.0)
                .map(|tbar| tbar.close / dbar.close)
        })
        .collect();
    if factors.is_empty() {
        return None;
    }
    factors.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if factors.len() >= CONSISTENCY_MIN_OVERLAP {
        let p10 = factors[factors.len() / 10];
        let p90 = factors[factors.len() * 9 / 10];
        if p10 <= 0.0 || p90 / p10 > SCALE_TOL {
            return None;
        }
    }
    let mid = factors.len() / 2;
    let median = if factors.len() % 2 == 0 {
        (factors[mid - 1] + factors[mid]) / 2.0
    } else {
        factors[mid]
    };
    (median.is_finite() && median > 0.0).then_some(median)
}

/// Robust `median(trusted_close / depth_close)` over only the most recent
/// overlapping buckets, used to sanity-check trusted bars against an independent
/// corroborator. Unlike [`chart_depth_source_scale_factor`] this ignores deep
/// history — where an unadjusted depth source legitimately sits on a different
/// scale per split era — and accepts the scale only when that recent window is
/// internally tight (p75/p25 within `LOCAL_TOL`). That lets it anchor an outlier
/// check on a clean recent scale without being thrown off by old unadjusted
/// bars, so a transient bad print in the trusted feed can be caught and the
/// genuine deep-history splice (handled separately) is left alone.
///
/// Note Kraken xStock bars are sourced from Alpaca on the backend, so the
/// trusted tier (kraken-equities + alpaca) is not self-corroborating — a backend
/// mis-adjustment hits both identically. Yahoo is the independent reference.
fn chart_recent_overlap_scale(
    trusted: &std::collections::BTreeMap<i64, Bar>,
    depth: &std::collections::BTreeMap<i64, Bar>,
) -> Option<(f64, i64)> {
    const MIN_COUNT: usize = 40;
    const MIN_OVERLAP: usize = 10;
    const LOCAL_TOL: f64 = 1.25;
    // The adjudication window is defined by TIME, not a fixed bucket count. A flat
    // 40 buckets is ~40 days on D1 but only ~10 hours on M15, so an intraday bad
    // print even a day old was never reached (the WOK M15 artifact). Take the most
    // recent buckets covering at least MIN_COUNT *and* at least RECENT_WINDOW_MS.
    // The p25/p75 tightness check below still rejects any window that straddles a
    // split-era scale change, so widening the reach stays safe.
    const RECENT_WINDOW_MS: i64 = 30 * 24 * 60 * 60 * 1000; // 30 days

    // Newest-first shared buckets, taken until BOTH the count and time floors pass.
    let mut recent: Vec<(i64, f64)> = Vec::new();
    let mut time_floor: Option<i64> = None;
    for (bucket, ratio) in trusted.iter().rev().filter_map(|(bucket, tbar)| {
        depth
            .get(bucket)
            .filter(|dbar| tbar.close > 0.0 && dbar.close > 0.0)
            .map(|dbar| (*bucket, tbar.close / dbar.close))
    }) {
        match time_floor {
            Some(floor) if recent.len() >= MIN_COUNT && bucket < floor => break,
            None => time_floor = Some(bucket - RECENT_WINDOW_MS),
            _ => {}
        }
        recent.push((bucket, ratio));
    }
    if recent.len() < MIN_OVERLAP {
        return None;
    }
    let window_start = recent.iter().map(|(bucket, _)| *bucket).min()?;
    let mut ratios: Vec<f64> = recent.iter().map(|(_, ratio)| *ratio).collect();
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Reject a noisy recent window: the tight middle band must be consistent so
    // we never anchor the outlier check on a mixed-scale (mid-split) overlap.
    let p25 = ratios[ratios.len() / 4];
    let p75 = ratios[ratios.len() * 3 / 4];
    if p25 <= 0.0 || p75 / p25 > LOCAL_TOL {
        return None;
    }
    let mid = ratios.len() / 2;
    let median = if ratios.len() % 2 == 0 {
        (ratios[mid - 1] + ratios[mid]) / 2.0
    } else {
        ratios[mid]
    };
    (median.is_finite() && median > 0.0).then_some((median, window_start))
}

/// Process-wide primary broker for the equity data merge (ADR-126). The merge
/// runs on many background cache/load threads that do not carry app state, so the
/// single app-level `primary_broker` choice is mirrored here as an atomic that the
/// pure ranking core reads. 0 = Kraken (legacy default), 1 = Alpaca. The app
/// updates this whenever `primary_broker` changes (top-bar switch + session load).
static MERGE_PRIMARY_BROKER: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub(crate) fn set_chart_merge_primary_broker(primary: OrderBroker) {
    let encoded = match primary {
        OrderBroker::Kraken => 0,
        OrderBroker::Alpaca => 1,
    };
    MERGE_PRIMARY_BROKER.store(encoded, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn chart_merge_primary_broker() -> OrderBroker {
    match MERGE_PRIMARY_BROKER.load(std::sync::atomic::Ordering::Relaxed) {
        1 => OrderBroker::Alpaca,
        _ => OrderBroker::Kraken,
    }
}

/// The equity source tag that defines the trusted price scale under the current
/// primary broker — also the sole valid native source for low-TF (M1/M5) equity
/// merges. ADR-126.
pub(crate) fn chart_equity_native_source_tag() -> &'static str {
    chart_merge_primary_broker().equity_source_tag()
}

/// Source priority for the equity merge under the *current* primary broker
/// (reads the process-wide selection). See [`chart_equity_source_rank_for`].
pub(crate) fn chart_equity_source_rank(source: &str) -> Option<u8> {
    chart_equity_source_rank_for(source, chart_merge_primary_broker())
}

/// Pure ranking core. The primary broker's equity source defines the trusted
/// price scale (rank 0); the *other* tradeable broker is the trusted-tier assist
/// (rank 2 — still ≤ `TRUSTED_MAX_RANK`, so it corroborates/gap-fills but cannot
/// redefine the scale). Yahoo/default stay depth-only fallbacks (ranks 3/4).
/// Swapping which tradeable source is rank 0 vs 2 is the entire ADR-126 merge
/// inversion; the SCALE_CAP staleness guard (ADR-124) is symmetric, so it
/// protects the chosen scale in either orientation.
pub(crate) fn chart_equity_source_rank_for(source: &str, primary: OrderBroker) -> Option<u8> {
    if source == primary.equity_source_tag() {
        return Some(0);
    }
    match source {
        // The non-primary tradeable broker (the assist lane).
        "alpaca" | "kraken-equities" => Some(2),
        "yahoo-chart" => Some(3),
        "default" => Some(4),
        _ => None,
    }
}

pub(crate) fn chart_prefers_fresh_equity_source(symbol: &str) -> bool {
    let compact = normalize_market_data_symbol(symbol)
        .replace('/', "")
        .trim_end_matches(".EQ")
        .to_ascii_uppercase();
    !compact.is_empty()
        && compact.len() <= 8
        && compact
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.')
        && !compact.ends_with("USD")
        && !compact.ends_with("USDT")
        && !compact.ends_with("USDC")
        && !compact.ends_with("ZUSD")
}

pub(crate) fn chart_forming_bar_allowed(last_bar_ts_ms: i64, now_ms: i64, tf_ms: i64) -> bool {
    if last_bar_ts_ms <= 0 || now_ms <= 0 || tf_ms <= 0 {
        return false;
    }
    let current_bucket = now_ms / tf_ms * tf_ms;
    current_bucket > last_bar_ts_ms && current_bucket.saturating_sub(last_bar_ts_ms) <= tf_ms
}

pub(crate) const CHART_SOURCE_ORDER: [(&str, &str); 6] = [
    ("kraken", "Kraken"),
    ("kraken-equities", "Kraken Equities"),
    ("kraken-futures", "Kraken Futures"),
    ("alpaca", "Alpaca"),
    ("yahoo-chart", "Yahoo Chart"),
    ("default", "Default"),
];

pub(crate) fn cache_source_label(source: &str) -> &'static str {
    CHART_SOURCE_ORDER
        .iter()
        .find_map(|(key, label)| (*key == source).then_some(*label))
        .unwrap_or("Source")
}

// Source / cache key helpers (O(1) dedup, candidate for chart_sources.rs submodule extraction)
pub(crate) fn chart_merged_equity_cache_key(symbol: &str, timeframe: &str) -> String {
    let symbol = normalize_market_data_symbol(symbol)
        .replace('/', "")
        .trim_end_matches(".EQ")
        .to_ascii_uppercase();
    format!("merged:{symbol}:{timeframe}")
}

pub(crate) fn chart_equity_low_timeframe_requires_native_source(timeframe: &str) -> bool {
    matches!(timeframe, "1Min" | "5Min")
}

/// Serialize merged bars into the cache JSON payload, or `None` when there is
/// nothing worth persisting (no bars, or none with a valid timestamp).
fn chart_merged_bars_to_cache_json(bars: &[Bar]) -> Option<String> {
    if bars.is_empty() {
        return None;
    }
    let json: Vec<serde_json::Value> = bars
        .iter()
        .filter_map(|bar| {
            let timestamp = chrono::DateTime::from_timestamp_millis(bar.ts_ms)?.to_rfc3339();
            Some(serde_json::json!({
                "timestamp": timestamp,
                "open": bar.open,
                "high": bar.high,
                "low": bar.low,
                "close": bar.close,
                "volume": bar.volume,
            }))
        })
        .collect();
    if json.is_empty() {
        return None;
    }
    serde_json::to_string(&json).ok()
}

#[cfg(test)]
pub(crate) fn chart_persist_merged_equity_bars_to_cache(
    cache: &SqliteCache,
    symbol: &str,
    timeframe: &str,
    bars: &[Bar],
) -> Result<(), String> {
    let Some(json) = chart_merged_bars_to_cache_json(bars) else {
        return Ok(());
    };
    let key = chart_merged_equity_cache_key(symbol, timeframe);
    cache.put_bars(&key, &json)
}

/// Best-effort merged-cache warm for hot render-thread loads: skips the write
/// entirely when the writer connection is busy (bulk sync) so the render thread
/// never stalls behind it. The merged blob is re-materialised off-thread (the
/// background sync) when it ends up missing.
fn chart_persist_merged_equity_bars_best_effort(
    cache: &SqliteCache,
    symbol: &str,
    timeframe: &str,
    bars: &[Bar],
) {
    let Some(json) = chart_merged_bars_to_cache_json(bars) else {
        return;
    };
    let key = chart_merged_equity_cache_key(symbol, timeframe);
    let _ = cache.put_bars_if_uncontended(&key, &json);
}

#[cfg(test)]
pub(crate) fn chart_materialize_merged_equity_cache(
    cache: &SqliteCache,
    symbol: &str,
    timeframe: &str,
) -> Result<usize, String> {
    let merged = chart_build_merged_equity_bars_from_cache(cache, symbol, timeframe);
    chart_persist_merged_equity_bars_to_cache(cache, symbol, timeframe, &merged)?;
    Ok(merged.len())
}

pub(crate) fn chart_load_merged_equity_bars_from_cache(
    cache: &SqliteCache,
    symbol: &str,
    timeframe: &str,
) -> Vec<Bar> {
    let merged = chart_build_merged_equity_bars_from_cache(cache, symbol, timeframe);
    chart_persist_merged_equity_bars_best_effort(cache, symbol, timeframe, &merged);
    merged
}

#[cfg(target_os = "linux")]
fn chart_process_rss_mb() -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb = rest.split_whitespace().next()?.parse::<f64>().ok()?;
            return Some(kb / 1024.0);
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn chart_process_rss_mb() -> Option<f64> {
    None
}

fn chart_rss_label(rss_mb: Option<f64>) -> String {
    rss_mb
        .map(|rss| format!("{rss:.1} MB"))
        .unwrap_or_else(|| "n/a".to_string())
}

pub(crate) fn chart_log_merged_cache_load_start(
    log: &mut std::collections::VecDeque<LogEntry>,
    context: &str,
    symbol: &str,
    timeframe: &str,
) -> (std::time::Instant, Option<f64>) {
    let rss = chart_process_rss_mb();
    let msg = format!(
        "Merged cache load start ({context}): {symbol} [{timeframe}] rss={}",
        chart_rss_label(rss)
    );
    log.push_back(LogEntry::info(msg));
    (std::time::Instant::now(), rss)
}

pub(crate) fn chart_log_merged_cache_load_done(
    log: &mut std::collections::VecDeque<LogEntry>,
    context: &str,
    symbol: &str,
    timeframe: &str,
    bars: usize,
    started_at: std::time::Instant,
    rss_before_mb: Option<f64>,
) {
    let rss_after_mb = chart_process_rss_mb();
    let msg = format!(
        "Merged cache load done ({context}): {bars} bars for {symbol} [{timeframe}] load_ms={:.2} rss={} → {}",
        started_at.elapsed().as_secs_f64() * 1000.0,
        chart_rss_label(rss_before_mb),
        chart_rss_label(rss_after_mb)
    );
    log.push_back(LogEntry::info(msg));
}

fn chart_build_merged_equity_bars_from_cache(
    cache: &SqliteCache,
    symbol: &str,
    timeframe: &str,
) -> Vec<Bar> {
    if timeframe == "4Hour" {
        let hourly = chart_build_merged_equity_bars_from_cache(cache, symbol, "1Hour");
        if hourly.len() >= 2 {
            let hourly_raw = chart_bars_to_raw(hourly);
            let four_hour = chart_aggregate_raw_to_4hour(&hourly_raw);
            if four_hour.len() >= 2 {
                return chart_raw_to_bars(four_hour);
            }
        }
    }

    if timeframe == "1Week" {
        let daily = chart_build_merged_equity_bars_from_cache(cache, symbol, "1Day");
        if daily.len() >= 2 {
            let daily_raw = chart_bars_to_raw(daily);
            let weekly = chart_aggregate_raw_to_weekly(&daily_raw);
            if weekly.len() >= 2 {
                return weekly;
            }
        }
    }

    if timeframe == "1Month" {
        let daily = chart_build_merged_equity_bars_from_cache(cache, symbol, "1Day");
        if daily.len() >= 2 {
            let daily_raw = chart_bars_to_raw(daily);
            let monthly = ChartState::aggregate_daily_raw_to_monthly(daily_raw);
            if monthly.len() >= 2 {
                return monthly;
            }
        }
    }

    type RawBars = Vec<(i64, f64, f64, f64, f64, f64)>;
    let mut loaded: Vec<(&'static str, RawBars)> = Vec::new();
    // For equity/xStock M1/M5, only the PRIMARY broker's native rows are valid
    // merged inputs (ADR-126). The assist broker's / Yahoo's low-TF rows are stale
    // provider-assist artifacts unless explicitly selected by source override.
    let low_tf_native: [&'static str; 1] = [chart_equity_native_source_tag()];
    let broad_sources: [&'static str; 4] = ["yahoo-chart", "alpaca", "kraken-equities", "default"];
    let sources: &[&'static str] = if chart_equity_low_timeframe_requires_native_source(timeframe) {
        &low_tf_native
    } else {
        &broad_sources
    };
    for source in sources {
        for key in chart_source_cache_keys(source, symbol, timeframe) {
            let Ok(Some(raw)) = cache.get_bars_raw(&key) else {
                continue;
            };
            if raw.is_empty() {
                continue;
            }
            loaded.push((source, raw));
            break;
        }
    }

    // Yahoo exposes no native 4-hour interval (see `yahoo_chart_supports_timeframe`),
    // so a "4Hour" merge would otherwise have no independent corroborator and the
    // trusted-tier outlier correction is skipped entirely — exactly why a bad
    // Alpaca 4Hour print (WOK, 2026-06) reached the H4 chart while H1, corroborated
    // by Yahoo's 1h series, stayed clean. Synthesize a 4-hour Yahoo series by
    // aggregating cached 1-hour Yahoo bars to restore that corroborator.
    if timeframe == "4Hour" && !loaded.iter().any(|(src, _)| *src == "yahoo-chart") {
        if let Some(hourly) = chart_source_cache_keys("yahoo-chart", symbol, "1Hour")
            .iter()
            .find_map(|key| cache.get_bars_raw(key).ok().flatten())
            .filter(|raw| !raw.is_empty())
        {
            let agg = chart_aggregate_raw_to_4hour(&hourly);
            if agg.len() >= 2 {
                loaded.push(("yahoo-chart", agg));
            }
        }
    }

    let views: Vec<(&str, &[(i64, f64, f64, f64, f64, f64)])> = loaded
        .iter()
        .map(|(source, raw)| (*source, raw.as_slice()))
        .collect();
    let splits = chart_known_splits_from_cache(cache, symbol);
    chart_merge_equity_raw_bars(timeframe, &views, &splits)
}

fn chart_bars_to_raw(bars: Vec<Bar>) -> Vec<(i64, f64, f64, f64, f64, f64)> {
    bars.into_iter()
        .map(|bar| {
            (
                bar.ts_ms, bar.open, bar.high, bar.low, bar.close, bar.volume,
            )
        })
        .collect()
}

fn chart_raw_to_bars(raw: Vec<(i64, f64, f64, f64, f64, f64)>) -> Vec<Bar> {
    raw.into_iter()
        .map(|(ts_ms, open, high, low, close, volume)| Bar {
            ts_ms,
            open,
            high,
            low,
            close,
            volume,
        })
        .collect()
}

/// Aggregate a finer raw OHLCV series into 4-hour buckets aligned exactly to
/// [`chart_merge_bucket_ts`]'s "4Hour" boundaries, so the result overlaps native
/// 4-hour bars bucket-for-bucket inside the merge. Open = first bar in a bucket,
/// close = last, high/low = extremes, volume = sum. Used to synthesize a 4-hour
/// Yahoo corroborator from cached 1-hour Yahoo bars.
fn chart_aggregate_raw_to_4hour(
    raw: &[(i64, f64, f64, f64, f64, f64)],
) -> Vec<(i64, f64, f64, f64, f64, f64)> {
    let mut sorted: Vec<(i64, f64, f64, f64, f64, f64)> = raw
        .iter()
        .copied()
        .filter(|(ts, o, h, l, c, _v)| {
            *ts > 0
                && *o > 0.0
                && *h > 0.0
                && *l > 0.0
                && *c > 0.0
                && o.is_finite()
                && h.is_finite()
                && l.is_finite()
                && c.is_finite()
                && *h >= *l
        })
        .collect();
    sorted.sort_by_key(|(ts, ..)| *ts);

    let mut out: std::collections::BTreeMap<i64, Bar> = std::collections::BTreeMap::new();
    for (ts, o, h, l, c, v) in sorted {
        let bucket = chart_merge_bucket_ts("4Hour", ts);
        out.entry(bucket)
            .and_modify(|b| {
                if h > b.high {
                    b.high = h;
                }
                if l < b.low {
                    b.low = l;
                }
                b.close = c;
                b.volume += v;
            })
            .or_insert(Bar {
                ts_ms: bucket,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: v,
            });
    }
    out.into_values()
        .map(|b| (b.ts_ms, b.open, b.high, b.low, b.close, b.volume))
        .collect()
}

fn chart_aggregate_raw_to_weekly(raw: &[(i64, f64, f64, f64, f64, f64)]) -> Vec<Bar> {
    let mut sorted: Vec<(i64, f64, f64, f64, f64, f64)> = raw
        .iter()
        .copied()
        .filter(|(ts, o, h, l, c, _v)| {
            *ts > 0
                && *o > 0.0
                && *h > 0.0
                && *l > 0.0
                && *c > 0.0
                && o.is_finite()
                && h.is_finite()
                && l.is_finite()
                && c.is_finite()
                && *h >= *l
        })
        .collect();
    sorted.sort_by_key(|(ts, ..)| *ts);

    let mut out: std::collections::BTreeMap<i64, Bar> = std::collections::BTreeMap::new();
    for (ts, o, h, l, c, v) in sorted {
        let bucket = chart_merge_bucket_ts("1Week", ts);
        out.entry(bucket)
            .and_modify(|b| {
                b.high = b.high.max(h).max(c);
                b.low = b.low.min(l).min(c);
                b.close = c;
                b.volume += v;
            })
            .or_insert(Bar {
                ts_ms: bucket,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: v,
            });
    }
    out.into_values().collect()
}
