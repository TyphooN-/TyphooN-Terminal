# ADR-124: Depth-Era Promotion Must Not Redefine the Price Scale

**Status:** Accepted | **Date:** 2026-06-14

Companion to **ADR-113** (Cross-Source Equity Bar Merge & Data Integrity) and
**ADR-122** (Curated Stock-Split Fallback). Governs
`chart_reconcile_depth_split_adjustment` in `typhoon-native/src/app/chart.rs`.

## Context

ADR-113 added `chart_reconcile_depth_split_adjustment`: when a depth source
(Yahoo) agrees with the trusted tier recently but exposes **≥2 older stable
divergent eras**, it overwrites those trusted buckets with the depth OHLC. The
intent was the case where Kraken+Alpaca are both raw/mis-adjusted across reverse
splits and Yahoo is the only adjusted reference.

That promotion was applied **unconditionally** once two stable eras were found.
The "≥2 stable divergent eras" signal is **symmetric**, though — it looks
identical whether:

- trusted is raw and depth is the adjusted reference (promote = correct), or
- depth is back-adjusted onto a *runaway* scale and trusted is the compact,
  real-price source (promote = corrupting).

**WOK is the second case.** Verified against the live cache:

- WOK did **two 1-for-100 reverse splits** (ex-dates ~Oct/Nov 2025 and
  2025-12-29); the `yahoo/alpaca` close ratio steps `10000 → 100 → 1`.
- WOK has **no `kraken-equities`** cache, so the only trusted source is Alpaca —
  and for this microcap Alpaca's `adjustment=all` does **not** know the splits, so
  Alpaca is **raw** (compact traded prices, ~0.027 → 3.4 ×100 step at each split).
- Yahoo **is** back-adjusted across both splits, which compounds to **~10,000×**
  the recent price in deep history (Alpaca ~3.6 in 2024 ↔ Yahoo ~36,000).

Yahoo's per-split eras are individually stable, so the reconciliation pasted
Yahoo's tens-of-thousands-priced bars over Alpaca's compact ones. Because the era
structure depends on bar **density**, it fired differently per timeframe (3 eras
on 1Day, 24 on 1Hour), so the **same symbol rendered inconsistently** — compact
on D1/W1, spiked on H1/H4 (and 4Hour, which aggregates 1Hour). The depth *splice*
already dropped Yahoo here via its `SCALE_TOL` consistency guard
(`chart_depth_source_scale_factor`); only the **reconciliation override** bypassed
that guard.

This is the same class of failure ADR-113 set out to prevent ("do not splice
scale-jumped bars / do not lie on the chart"), reached through the one path that
skipped the scale check.

## Decision

The trusted tier **defines the price scale** (ADR-113). Depth may smooth a
mis-adjusted *continuity* but must never **relocate** bars by orders of magnitude.
Gate the promotion on `chart_depth_promotion_keeps_trusted_scale`:

- Promote only while the depth source stays within **`SCALE_CAP` (50×)** of its
  own recent (consensus-window) level across **every** divergent era. A genuine
  adjusted reference on a compact multi-split history passes; a runaway
  back-adjusted source (WOK/Yahoo, ~10,000×) is refused.
- When refused, the merge keeps the trusted bars unchanged. The splice's existing
  `SCALE_TOL` drop already keeps the runaway depth source out of the gap-fill, so
  the result is the **compact trusted series, identical across every timeframe**.

`50×` admits up to roughly two stacked ~1-for-7 reverse splits as a real
multi-era range while firmly rejecting the multi-`×100` back-adjustment runaway
(WOK is ~10,000×, two orders past the cap).

## Consequences

- WOK (and any symbol whose depth source is back-adjusted onto a runaway scale)
  stays on the compact traded scale on **all** timeframes — H1/H4/4Hour now match
  D1/W1/MN1. Verified end-to-end against the live cache: WOK 1Day and 1Hour both
  merge to `close ∈ [0.027, 8.0]` with **zero** `>20` bars (previously
  `maxHigh ≈ 84,960`).
- Conservative by design: a depth source that legitimately needs to correct more
  than `SCALE_CAP` is now skipped rather than promoted, leaving the trusted (raw)
  bars in place. Such symbols are better served by the **exact** back-adjust
  (ADR-122) once their splits are curated/populated — the robust cause-fix —
  rather than by cross-source era inference.
- Merged bars rebuild from provider rows on every load
  (`chart_load_merged_equity_bars_from_cache`), so stale pre-fix `merged:WOK:*`
  rows self-heal on the next symbol load; no manual cache purge.
- Tests: `chart_equity_merge_keeps_compact_trusted_scale_over_exploded_depth`
  reproduces the WOK two-split runaway and asserts the compact scale is kept;
  `chart_equity_merge_uses_adjusted_yahoo_for_multi_split_history` was retargeted
  to a **compact** multi-split case (the original asserted a 10,000× promotion —
  exactly the WOK pathology — and was incorrect).

## Note on the data model (correcting a prior mental model)

The deep-history scale gap for WOK is **~10,000×**, but the direction is the
opposite of "Yahoo is unadjusted": here **Yahoo is *over*-adjusted** (correctly
back-adjusted across two reverse splits, hence runaway) and **Alpaca is the
*unadjusted* one**. The magnitude matches earlier notes; the cause does not.
Either way the merge must not let a depth source redefine the trusted scale.
