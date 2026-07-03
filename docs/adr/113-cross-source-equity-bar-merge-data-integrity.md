# ADR-113: Cross-Source Equity Bar Merge & Data Integrity

**Status:** Accepted | **Date:** 2026-06-11

Companion to **ADR-112** (which sources feed the merge) and **ADR-103** (provider
lanes). Governs `chart_merge_equity_raw_bars` in `typhoon-native/src/app/chart.rs`.

## Context

The chart/research series for an equity is a `merged:SYMBOL:TF` blend of multiple
provider caches. Each provider adjusts (or fails to adjust) for corporate actions
differently, and thin microcaps get bad prints. Two failure modes must be handled
without lying on the chart:

1. **Scale discontinuities in deep history.** Yahoo's raw close is frequently
   *unadjusted* across splits/redenominations (WOK was ~10,000× too high before
   its 2025 action). A naïve splice pastes that straight onto the trusted scale.
2. **A bad recent print from the trusted source.** On 2026-06-09/10, Alpaca
   reported WOK at **~0.20 — exactly 2× reality** (Yahoo and TradingView, both
   adjusted, showed ~0.094) for two days. Decoding the cache showed
   `merged:WOK:*` was byte-identical to `alpaca:WOK:*`: the bad print propagated
   to every timeframe, pinned the autoscale, and poisoned every MA/ATR.

The second mode is the dangerous one, and it exposed an architectural gap:

- The merge has a **trusted tier** (rank ≤ 2: `kraken-equities` and `alpaca`; the
  rank table keeps a vestigial `tastytrade` slot only so legacy cached rows from
  the removed broker still merge) that *defines the price scale*, and a **depth tier** (`yahoo-chart`,
  `default`) that only *fills gaps* the trusted tier lacks. So a bad trusted bar
  is never challenged — the depth tier can't overwrite a bucket the trusted tier
  already owns.
- **Kraken sources stocks from Alpaca on the backend** (ADR-112). So the trusted
  tier is **not self-corroborating**: a backend mis-adjustment hits
  Kraken-equities and Alpaca identically. The only independent reference is
  Yahoo (and the live tape).
- The bad print was **not an adjustment discrepancy** — Alpaca (`adjustment=all`)
  and Yahoo *agreed* every day through 06-08, then Alpaca alone doubled. No
  adjustment scheme catches "one provider emitted a value the others don't
  corroborate." Only cross-source corroboration does.

Why "fetch raw everywhere and adjust ourselves" was rejected as the fix: it does
not catch this failure (bad raw prints still happen), it requires a *trustworthy*
corporate-actions feed which does not exist for microcaps (the bug just moves to
"whose CA data do we trust"), and it is a multi-week build. It remains a possible
future north star (store raw OHLCV + a CA-factor table, derive adjusted on
demand) but is not required to fix data integrity today.

## Decision

### 1. Keep the trusted/depth tier model and deep-history back-adjust

- Trusted tier defines scale; best rank wins per bucket.
- Depth sources fill only buckets the trusted tier lacks, **back-adjusted** to
  the trusted scale by `median(trusted_close / depth_close)` over their overlap
  (`chart_depth_source_scale_factor`).
- A depth source whose overlap ratios are internally inconsistent (p90/p10 >
  `SCALE_TOL`) — the tell-tale of an unadjusted action mid-history — is
  **dropped entirely** rather than splicing scale-jumped bars.

### 2. NEW: trusted-tier recent-window outlier guard

Before the depth splice, validate the trusted bars themselves against an
independent corroborator:

- Compute a **recent-window** scale `median(trusted_close / depth_close)` over
  only the most recent overlapping buckets (`chart_recent_overlap_scale`), and
  accept it only when that window is internally tight (p75/p25 within
  `LOCAL_TOL`). This deliberately ignores deep history, where an unadjusted depth
  source legitimately sits on a different per-era scale.
- For buckets **within that recent window**, any trusted bar that diverges from
  the rescaled corroborator by more than `OUTLIER_RATIO` (1.5×) is replaced with
  the corroborated value.
- Only the **best valid corroborator** adjudicates. Correction is **never**
  applied outside the recent window (a regression caught in test: applying the
  recent scale to deep history "corrected" good Alpaca bars to Yahoo's 1000×
  garbage).

This is the same philosophy as the existing back-adjust — *validate the scale* —
extended from deep-history splices to the **recent anchor** bars.

### 3. Independent corroborator must stay enabled

Because the trusted tier is not self-corroborating (Kraken == Alpaca backend),
the guard needs at least one independent source. Yahoo is that source today;
keep it (or another independent lane) enabled for equities. The live last-trade
tick, where available, is an even better recent anchor and should feed the guard
in future.

### 4. Yahoo `adjclose` direction (companion improvement)

Switch Yahoo ingestion from raw `close` to `adjclose` (split+dividend adjusted,
what TradingView shows), scaling O/H/L by each bar's `adjclose/close` ratio.
This makes Yahoo a *clean, scale-consistent corroborator across the entire
series* (not just the recent window), retires most of the deep-history
back-adjust hack, and lets the guard protect old bars too. Requires a Yahoo
equity-history re-sync.

## Regression guards (do-not list)

- **Do not** trust a single provider's recent bar without cross-source
  corroboration — the trusted tier defines scale but is not infallible, and
  Kraken+Alpaca are the same backend.
- **Do not** correct deep history using a recent-window scale — corporate-action
  eras have different legitimate scales.
- **Do not** treat Kraken-equities and Alpaca as independent confirmations of
  each other.
- **Do not** disable the last independent corroborator (Yahoo) for equities
  without replacing it.

## Consequences

- A transient 1.5×+ bad print from any single trusted provider on a recent bar is
  corrected to consensus instead of charted — protecting *every* symbol, not
  just WOK.
- Deep-history splicing and unadjusted-source dropping are unchanged.
- Tests: `chart_equity_merge_corrects_trusted_outlier_print_against_recent_corroborator`
  reproduces the WOK case; the existing drop/back-adjust tests still pass
  (the guard correctly does not fire when the recent overlap is mixed-scale).

## Status of implementation (2026-06-11)

- **Done:** recent-window outlier guard + helper + test
  (`chart_merge_equity_raw_bars`, `chart_recent_overlap_scale`).
- **Done:** Yahoo `adjclose` ingestion (§4) rebases each Yahoo bar onto the
  split/dividend-adjusted scale before it participates in corroboration.
- **Open future work (deliberately deferred):** live-tick anchor for the
  recent-window guard. Wiring live quotes into the merge guard touches the
  merge path, where changes carry outsized data-integrity risk (the same
  reason the merge-dedup consolidation was skipped); revisit only with a
  reproducible case the bar-based corroborator anchor gets wrong.
- **General split coverage (2026-07-03):** `research_stock_splits` is now fed
  by the bulk scrape for every symbol via the combined FMP + keyless-Yahoo
  fetcher (ADR-122), strengthening the exact back-adjust leg this ADR's
  merge hierarchy relies on.
- **Operator action, not code:** one-time purge of already-cached bad WOK
  2026-06-09/10 `merged:WOK:*` rows is intentionally not auto-run against the
  live cache; rebuild/restart self-heals on next merge, and manual SQL deletion
  remains available if immediate cleanup is explicitly approved.
