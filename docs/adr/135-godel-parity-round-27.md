# ADR-135: Godel Parity Round 27 — OMEGA / DFA / BURKE / MONTHSEAS / ROLLSPRD

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-134
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| OMEGA | No | No | Yes | Yes | No (deferred — ADR-188) |
| DFA | No | No | Yes | Yes | No (deferred — ADR-188) |
| BURKE | No | No | Yes | Yes | No (deferred — ADR-188) |
| MONTHSEAS | No | No | Yes | Yes | No (deferred — ADR-188) |
| ROLLSPRD | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical econometric primitives (Omega ratio, detrended fluctuation analysis, Burke ratio, monthly calendar seasonality, Roll implicit bid-ask spread) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 26 (ADR-134) shipped the core drawdown/liquidity/normality triad
— Calmar, Ulcer, Variance Ratio, Amihud, Jarque-Bera. The HP-local
surface now carries the standard textbook risk/return ratios,
moment/distribution tests, drawdown-adjusted returns, microstructure
price impact, and random-walk formal testing.

Five distinct-view surfaces remain that draw from the existing HP
cache but capture axes none of the 123 shipped sub-blocks covers:

1. **No distribution-free ratio that uses the full return shape.**
   SHARPR uses moments (mean/std). DOWNVOL/Sortino uses only the
   negative tail. CALMAR uses only max drawdown. But none partitions
   the *full* return distribution into gains vs losses without moment
   assumptions. The Omega ratio `E[max(r-τ,0)] / E[max(τ-r,0)]` at
   τ=0 is exactly that: a moment-free gain/loss integral ratio that
   captures the entire distributional shape (not just the first two
   moments).

2. **No robust-to-non-stationarity Hurst estimator.** HURST uses
   rescaled-range (R/S) analysis, which assumes the underlying series
   is stationary. Real return series have regime shifts, volatility
   clustering, and trend drift — all of which bias R/S. Detrended
   Fluctuation Analysis (DFA, Peng et al. 1994) computes the same
   persistence exponent α using windowed detrended residuals, and is
   robust to non-stationarity. Having both HURST and DFA lets an
   agent cross-check persistence claims — DFA α ≠ HURST H is a signal
   the series is non-stationary in a way that invalidates either
   estimator alone.

3. **No event-weighted drawdown-adjusted return.** CALMAR uses only
   the max drawdown. ULCER uses the RMS of *all* drawdown points
   continuously. Between these extremes sits the Burke ratio:
   `return / sqrt(Σ dd_i²)` where `dd_i` is the magnitude of each
   *distinct* drawdown episode (peak-to-trough-to-recovery). Burke
   weights by the top-k worst completed episodes, which is the view
   most practitioners actually care about — "how bad are my worst 3
   drawdowns, not just the single deepest one?"

4. **No calendar axis.** The entire packet is return-axis or
   price-axis; no surface looks at *when* in the calendar returns
   historically show up. "Sell in May," "January effect," "Santa
   rally," "summer doldrums" are canonical trading folklore backed
   by long-horizon research. A monthly hit-rate snapshot (share of
   historical years each calendar month closed positive) makes this
   calendar axis visible to the agent.

5. **No implicit bid-ask spread.** AMIHUD captures price impact per
   dollar traded. But microstructure has a second foundational
   scalar: the *implicit effective spread* from Roll (1984). It
   exploits the fact that bid/ask bounce induces negative first-lag
   autocorrelation in consecutive price changes: `spread = 2·√(-Cov(Δp_t, Δp_{t-1}))`.
   When cov is negative (bid/ask bounce dominates), Roll gives a
   clean closed-form effective spread in bps. When cov is
   non-negative (trending dominates bounce), the model fails
   identifiably — which is itself information.

Round 27 ships these five surfaces as ADR-135. Same additive envelope
as Rounds 5–26: no new fetchers, no cross-symbol scans, no new
external API dependencies. OMEGA / DFA / BURKE / ROLLSPRD compute
from the trailing 253-session window on the existing HP cache.
MONTHSEAS uses the *full* HP cache (not windowed) because calendar
seasonality requires multi-year history.

## Decision

Ship Round 27 as a five-surface additive bundle using schema v28
layered on v27:

| Surface    | Table                 | Purpose                                                      |
|------------|-----------------------|--------------------------------------------------------------|
| OMEGA      | `research_omega`      | Omega ratio at threshold 0 — distribution-partition ratio     |
| DFA        | `research_dfa`        | Detrended Fluctuation Analysis — Hurst alternative            |
| BURKE      | `research_burke`      | Burke ratio — event-weighted drawdown-adjusted return         |
| MONTHSEAS  | `research_monthseas`  | Monthly seasonality hit rate + mean return per calendar month |
| ROLLSPRD   | `research_rollsprd`   | Roll's (1984) implicit bid-ask spread in bps                  |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (5 buckets +
`INSUFFICIENT_DATA` sentinel). Label strings:

- **OMEGA**: `VERY_POOR` (Ω<0.5) / `POOR` (<0.9) / `NEUTRAL` (<1.1)
  / `GOOD` (<1.5) / `EXCELLENT` (≥1.5 or ∞)
- **DFA**: `ANTI_PERSISTENT` (α<0.35) / `MEAN_REVERTING` (<0.45)
  / `RANDOM_WALK` (<0.55) / `PERSISTENT` (<0.65) / `STRONGLY_PERSISTENT` (≥0.65)
- **BURKE**: `VERY_POOR` (<-0.5) / `POOR` (<0) / `NEUTRAL` (<0.5)
  / `GOOD` (<1.5) / `EXCELLENT` (≥1.5); `EXCELLENT` on no-event with
  positive return
- **MONTHSEAS**: `STRONG_SEASONAL` (best-worst hit spread ≥40%)
  / `MILD_SEASONAL` (≥25%) / `NEUTRAL` (≥15%) / `INCONSISTENT` (<15%)
- **ROLLSPRD**: `TIGHT` (<10 bps) / `NORMAL` (<30) / `WIDE` (<75)
  / `VERY_WIDE` (≥75); `INVALID_POSITIVE_COV` when first-lag cov ≥ 0

## Consequences

### Positive

- **Distribution-free ratio added (OMEGA).** First packet surface
  that uses the full return distribution without moment
  assumptions. SHARPR/DOWNVOL/CALMAR are now complemented by a
  ratio that works on fat-tailed, asymmetric, or skewed returns
  where moment-based measures can mislead.
- **Persistence cross-check (DFA).** Having both R/S-based HURST
  and detrended-residual DFA means agents can flag non-stationary
  series as those where the two disagree.
- **Drawdown gradient complete.** CALMAR (max-only) + ULCER (all-dd
  continuous) + BURKE (event-weighted) span the full spectrum. Each
  weights drawdowns differently — an agent can triangulate whether
  "this drawdown profile is driven by one catastrophic event, a
  continuous grind, or a few discrete bad episodes."
- **Calendar axis unlocked.** MONTHSEAS gives agents a view into
  "Sell in May," "Santa rally," etc. — and lets them detect when a
  ticker has unusually strong or weak calendar effects vs. its peer
  set. No other packet surface touches this axis.
- **Microstructure spread complete.** AMIHUD (Round 26) covers
  price-impact-per-dollar; ROLLSPRD covers effective-spread-in-bps.
  The two together give the reader both foundational liquidity
  scalars.

### Negative / Risks

- **Schema migration.** `create_research_tables_v28` is additive
  over v27, so peers on v27 who receive v28 rows through LAN sync
  will create the 5 new tables via the existing create-before-insert
  path. No back-compat break; the initialization just adds 5 new
  tables and 5 new indexes.
- **MONTHSEAS is full-HP-scan, not 253-window.** This is
  intentional — meaningful seasonality requires 5+ years of monthly
  data, and the 253-bar window is only ~1 year. But it means
  MONTHSEAS compute is O(N) over the full HP cache rather than
  O(253), still cheap but a different O-profile from the others.
- **DFA requires ≥100 bars** vs the usual ≥30. The four-scale log-log
  fit is unstable with fewer bars. Symbols with <100 HP bars will
  return `INSUFFICIENT_DATA`, which is correct but an additional
  gate to track vs HURST (which gates at ≥50).
- **ROLLSPRD's `INVALID_POSITIVE_COV` branch** will fire on
  trending/momentum symbols where bid/ask bounce does not dominate
  the short-term dynamics. This is the model failing *correctly* —
  we report the label instead of emitting a bogus spread — but
  consumers should know the branch exists and not treat its absence
  as a bug.
- **Packet weight.** Each surface adds roughly 300–900 bytes per
  symbol (MONTHSEAS is the heaviest due to the 12-month grid).
  Updated envelope numbers appear in the RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established
  in Rounds 24–26 (UP=green, DOWN=red, AXIS_TEXT=neutral). For each
  window, the color mapping is defined by the label-to-color match
  in `native/src/app.rs`.
- **All five surfaces use the same broker handler shape** that has
  been stable since Round 22: `BrokerCmd::Compute*Snapshot →
  tokio::spawn → compute → msg_tx.send(BrokerMsg::*SnapshotMsg)`.
  The receive arm in `update()` both updates UI state (if the
  current view's symbol matches) and upserts to the SQLite cache.
  LAN sync takes it from there.
- **Palette aliases** avoid all prior bindings. Notably `SEASONALITY`
  already belongs to SEAG (Round 13), so MONTHSEAS uses only `SEAS`
  / `MONTH_SEAS` / `MONTHLY_SEASONALITY` / `MONTHLYSEASONALITY`
  aliases. Similarly `HURST` is unchanged; DFA uses
  `DETRENDED_FLUCT` / `DFAALPHA` aliases.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 383
  passing (up from 367 in Round 26, +16 new: 5 roundtrip + 11
  compute tests).
- `cargo check -p typhoon-native` — clean.
- Each new surface carries at least one "rising series" or
  "bouncing series" deterministic-fixture compute test that
  validates the label path beyond just the `INSUFFICIENT_DATA`
  sentinel.

## Packet envelope

After Round 27, single-symbol packet target envelope is **~50-98 KB**
(up from 48-94 in Round 26). Basket (10 symbols via BASKET) is
**~490-980 KB** (up from 470-940). Sub-block count grows 123 → 128.

Total HP-local research snapshot count after Round 27: **87**
(82 + 5). Total cross-symbol rank snapshots unchanged.
