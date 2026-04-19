# ADR-151: TA-Lib + Godel Parity Round 42 — SQUEEZE / SQUEEZERANK / BBSQUEEZE / DONCHIAN / KAMA

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-150
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| SQUEEZE | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| SQUEEZERANK | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| BBSQUEEZE | Canonical (all terminals) | Yes (`BBANDS`) | Yes | Yes | No (deferred — ADR-188) |
| DONCHIAN | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| KAMA | Yes | Yes (`KAMA`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** mixed — Godel-documented composite short-squeeze score and rank watchlist; canonical technical overlays (Bollinger-band squeeze via TA-Lib `BBANDS`, Donchian N-bar channel, Kaufman Adaptive MA `KAMA`).

## Context

Round 41 (ADR-150) shipped MCLEODLI/OUFIT/GPH/BURGSPEC/KENDALLTAU, taking
HP-local research surfaces to 157 and per-symbol sub-blocks to 198.

The user flagged that $CAR was being short-squeezed and asked whether we
have an outlier-detection system that can surface similar setups in the
future — both as an automated signal *and* as a dedicated watchlist UI.

Pre-implementation survey showed ~70% of the ingredients are already
cached:

- `ShortInterestSnapshot` (short % of float, days-to-cover, a
  coarse `squeeze_risk_label`)
- `IvolSnapshot` (iv_rank on 0..100)
- `RelVolSnapshot` (rel_volume_20d, 5d, 60d)
- Historical bars (20d momentum)

But there was **no composite fusion** across these axes, and **no
standalone watchlist** surface that scans the cache and sorts by
squeeze strength. Round 42 closes both gaps, plus three orthogonal
technical surfaces (Bollinger-band squeeze, Donchian breakout, KAMA
efficiency ratio) that the existing library did not carry.

1. **No composite short-squeeze score.** SQUEEZE fuses
   (short_percent_of_float, days_to_cover, 20d momentum, relvol_20d,
   iv_rank) by normalising each axis through a saturating curve to
   0..100 and taking a weighted mean (short-float + DTC carry 1.5×
   weight; momentum / relvol / IV-rank carry 1.0×). Labels:
   `NO_SQUEEZE` (<20) / `WATCH` (<40) / `ELEVATED` (<60) /
   `STRONG` (<80) / `EXTREME` (≥80) / `INSUFFICIENT_DATA`
   (<3 of 5 axes present).

2. **No cross-symbol squeeze rank.** SQUEEZERANK table-scans
   `research_squeeze` and percentile-ranks the subject's composite
   score against every symbol in the cache that has a SQUEEZE row.
   Labels: `TOP_1PCT` / `TOP_5PCT` / `TOP_10PCT` / `ABOVE_MEDIAN` /
   `BELOW_MEDIAN`.

3. **No standalone squeeze watchlist UI.** The new *SQUEEZE Watchlist*
   window issues a `RefreshSqueezeWatchlist` broker command that
   recomputes SQUEEZE + SQUEEZERANK across every symbol with a
   `research_short_interest` row, upserts the result, and returns the
   sorted top-N for on-screen display. User-invoked via
   `SQUEEZEWATCHLIST` / `SQZWATCH` aliases.

4. **No Bollinger-band width squeeze.** BBSQUEEZE computes 20-bar
   Bollinger width = (upper − lower) / mid and percentile-ranks the
   current width against its trailing 120-bar history. Labels:
   `TIGHT_SQUEEZE` (≤10th pct) / `MODERATE_SQUEEZE` (≤25th) /
   `NORMAL` / `EXPANSION` (≥90th).

5. **No Donchian-channel breakout detector.** DONCHIAN reports 20-bar
   upper/lower channels and evaluates the current close against the
   *prior* (excluding today) channel to detect genuine breakouts.
   Labels: `BREAKOUT_UP` / `APPROACH_UP` (pos ≥ 80%) / `NEUTRAL` /
   `APPROACH_DOWN` (pos ≤ 20%) / `BREAKOUT_DOWN`.

6. **No KAMA / efficiency-ratio trend filter.** KAMA is Kaufman's
   Adaptive Moving Average. We report the efficiency ratio
   ER = |close_t − close_{t−n}| / Σ|Δclose_i| at n=10, the last KAMA
   value (using standard fast=2 / slow=30 constants in a recursion
   seeded from the period-0 SMA), and the 5-bar KAMA slope %. Labels:
   `STRONG_TREND` (ER ≥ 0.7) / `MODERATE_TREND` / `WEAK_TREND` /
   `CHOPPY`.

Round 42 ships these as ADR-151. Same additive envelope as Rounds
5–41: no new fetchers, no new external API dependencies, all compute
from the HP cache + existing short-interest / IV / relvol caches.

## Decision

Ship Round 42 as a five-surface additive bundle using schema v43
layered on v42:

| Surface       | Table                   | Purpose                                                                |
|---------------|-------------------------|------------------------------------------------------------------------|
| SQUEEZE       | `research_squeeze`      | Composite short-squeeze score across 5 orthogonal axes                 |
| SQUEEZERANK   | `research_squeezerank`  | Cross-symbol percentile rank of SQUEEZE composites                     |
| BBSQUEEZE     | `research_bbsqueeze`    | Bollinger-band width squeeze detector (20-bar vs 120-bar history)      |
| DONCHIAN      | `research_donchian`     | 20-bar Donchian channel breakout detector                              |
| KAMA          | `research_kama`         | Kaufman Adaptive Moving Average + efficiency ratio                     |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (3–5 active buckets +
`INSUFFICIENT_DATA` sentinel). Label thresholds are documented above.

A new *SQUEEZE Watchlist* window is added as a **standalone UI**
(not a per-symbol research window). It refreshes by scanning the
entire `research_short_interest` table, recomputing SQUEEZE for every
symbol that also has HP bars, persisting the new rows, then
populating SQUEEZERANK across the updated set. UI sorts by composite
score desc and displays the top 50.

## Consequences

### Positive

- **First composite outlier signal.** SQUEEZE is the first surface
  that explicitly fuses short-interest mechanics + price action + IV
  positioning into a single label, rather than forcing the user to
  open five different windows and cross-reference.
- **First table-scan watchlist.** The SQUEEZE Watchlist is the first
  surface that gives the user a sorted on-screen list of *the riskiest
  symbols in the cache right now*, not "tell me about symbol X".
  Future rounds can generalise the pattern (e.g. breakout watchlists).
- **LAN sync propagates outlier signals.** SQUEEZE, SQUEEZERANK,
  BBSQUEEZE, DONCHIAN, KAMA all flow through `lan_sync` via the
  standard v43 path, so a watchlist computed on one terminal is
  visible on every peer after the next sync window — no cross-device
  recompute needed.
- **BBSQUEEZE closes a gap.** Volatility contraction is a canonical
  "compressed spring" setup; the bare BB overlay on the chart was
  visual-only and didn't carry a label or percentile rank.
- **DONCHIAN breakout is distinct from existing trend detectors.**
  TREND (ADR-??) and KAMA both characterize ongoing regime; Donchian
  explicitly tests "is the latest close outside the prior N-bar
  envelope?" which is a binary breakout event the other surfaces
  don't produce.
- **KAMA efficiency ratio adds a trend-vs-chop filter.** Distinct
  from HURST/DFA (which measure persistence of *returns*, not of
  *price direction*). ER measures the fraction of total move that
  was "useful" vs noise — the classic filter for "should I trust a
  trend-following signal right now?".
- **No new external dependencies, no fetcher expansion.** Pure
  compute on existing caches — same additive envelope as Rounds 26–41.

### Negative / Risks

- **Schema migration.** `create_research_tables_v43` is additive over
  v42; peers on v42 who receive v43 rows via LAN sync will create the
  5 new tables via the existing create-before-insert path. No
  back-compat break.
- **SQUEEZE weighting is a choice.** The 1.5× / 1.0× weight split
  reflects the observation that short-float and days-to-cover are the
  *causal* mechanics of a squeeze while momentum / relvol / IV-rank
  are triggers. Users looking for momentum-led setups might prefer a
  flatter weighting — documented in the ADR and left as a later
  configurability item (not in scope for Round 42).
- **Paid-API gaps remain for SQUEEZE.** Five *additional* axes that
  would sharpen the signal are paid-API-gated:
  - **Cost-to-borrow / borrow-fee** (S3 Partners, Ortex, Interactive
    Brokers premium) — rising CTB is often the single most predictive
    leading indicator.
  - **Failure-to-deliver (FTD) counts** (SEC Fail data is public but
    lagged 2 weeks; real-time is vendor-gated).
  - **Unusual options activity / call-put sweep ratios** (CBOE
    Livevol, Trade Alert) — needed to detect gamma-squeeze setups.
  - **Institutional holders concentration** (Fintel paid tier).
  - **Social-media mention surges** (Reddit / X sentiment, vendor).

  None of these are free; we noted them in the composite's `note`
  field for manual follow-up. Can be reconsidered if a paid data
  tier is approved.
- **SQUEEZERANK is global, not sector-bucketed.** A biotech outlier
  and a fintech outlier will compete against each other on the same
  rank. This is deliberate for v1 (keeps the scan simple) — sector
  bucketing can layer on later via a `SECTORSQUEEZERANK` surface.
- **BBSQUEEZE uses a fixed 20/120 pair.** User-configurable (10/60,
  20/120, 50/252) would be a future ergonomics win; the current
  fixed config matches the canonical Bollinger/Chaikin defaults.
- **Donchian breakout excludes the current bar from the prior
  channel.** Otherwise the current bar's high/low is guaranteed to
  equal the upper/lower channel and every close would "break out".
  This is the standard convention (see Turtle rules) — documented.
- **KAMA recursion is recursive and seed-dependent.** We seed from
  the period-0 SMA which is standard; alternate seeds (e.g. EMA or
  the very first close) produce slightly different values but converge
  to the same label after a few hundred bars.
- **Packet weight.** SQUEEZE adds ~320 bytes, SQUEEZERANK ~160,
  BBSQUEEZE ~240, DONCHIAN ~220, KAMA ~210. Total Round 42
  addition: ~1.15 KB/symbol. SQUEEZE Watchlist is a UI artifact and
  does not appear in the packet.

### Neutral

- **Label-based color scheme continues** the convention established
  in Rounds 24–41 (UP=green for "favorable" label, DOWN=red for
  "adverse", AXIS_TEXT=neutral). For SQUEEZE, DOWN=red marks STRONG /
  EXTREME because that's the *signal*; for KAMA, UP=green marks
  STRONG_TREND / MODERATE_TREND and DOWN=red marks CHOPPY.
- **Palette alias disambiguation.** Bare `SQUEEZE`, `DONCHIAN`,
  `KAMA`, `KAUFMAN` are already bound to chart-overlay toggles
  (indicator plots) — these are *visual* toggles for the main chart
  pane. Round 42 research windows use disambiguated aliases only
  (e.g. `SHORTSQUEEZE`, `SQZCOMP`, `DONCHIANBREAK`, `KAMAFIT`,
  `KAMA_ER`, etc.) to avoid shadowing the chart-overlay handlers.
- **All five surfaces use the same broker handler shape** that has
  been stable since Round 22. SQUEEZE and the Watchlist additionally
  upsert to the cache from inside the handler since they depend on
  multi-table reads — this is the pattern established for
  cross-symbol rank surfaces.

## Verification

- `cargo test -p typhoon-engine --lib` — target 1136 passing (up from
  1126 in Round 41, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; Round 42 field names use
  `_win` suffix to avoid collision with existing chart-overlay
  booleans (`show_squeeze`, `show_donchian`, `show_kama`).
- SQUEEZE/SQUEEZERANK/BBSQUEEZE/DONCHIAN/KAMA compute_oscillating
  use the ±0.5% oscillating fixture (150 bars). Each asserts
  label belongs to its regime set, scalars are finite when label
  is not INSUFFICIENT_DATA, and axis-specific invariants:
  SQUEEZE composite ∈ [0,100] with synthetic 5-axis input, all 5
  per-axis scores ∈ [0,100]; SQUEEZERANK small-peer-set (n<5) is
  INSUFFICIENT_DATA, full-peer (n=10) has rank ∈ [1, peer_count]
  and percentile ∈ [0,100]; BBSQUEEZE upper ≥ mid ≥ lower and
  min ≤ max; DONCHIAN upper ≥ lower and pos ∈ [0,100];
  KAMA efficiency_ratio ∈ [0,1].

## Packet envelope

After Round 42, single-symbol packet target envelope is **~68-135 KB**
(up from 67-134 in Round 41). Basket (10 symbols via BASKET) is
**~680-1350 KB** (up from 670-1340). Sub-block count grows 198 → 203.

Total HP-local research snapshot count after Round 42: **162**
(157 + 5). Total cross-symbol rank snapshots: +1 (SQUEEZERANK).
