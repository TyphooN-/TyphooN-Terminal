# ADR-136: Godel Parity Round 28 — PARKINSON / GKVOL / RSVOL / CVAR / DOWEFFECT

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-135
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 27 (ADR-135) shipped OMEGA/DFA/BURKE/MONTHSEAS/ROLLSPRD —
distribution-free ratio, persistence cross-check, event-weighted
drawdown-adjusted return, calendar month seasonality, and implicit
bid-ask spread. 128 HP-local research sub-blocks cover the return-,
drawdown-, distribution-, persistence-, liquidity-, and
monthly-calendar axes.

Five canonical surfaces remain, each mapping to an axis not covered
by the existing 128 sub-blocks:

1. **No range-based volatility estimator.** The packet has
   close-to-close vol via `CLOSEVOL`/`RVCONE`/`VOLOFVOL`, but no
   estimator that uses the daily H/L range. Parkinson's (1980)
   estimator `σ² = (1/(4·ln2·n)) · Σ(ln(H/L))²` is ~5.2× more
   statistically efficient than close-to-close and is the textbook
   first entry in any OHLC-vol family.

2. **No full-OHLC volatility estimator.** Garman-Klass (1980)
   `σ² = (1/n)·Σ[0.5·(ln H/L)² − (2ln2−1)·(ln C/O)²]` combines the
   H-L range with the C-O drift component for ~7.4× efficiency —
   the most commonly deployed range-vol estimator in practice.

3. **No drift-independent OHLC estimator.** Both Parkinson and
   Garman-Klass assume zero drift. Rogers-Satchell (1991)
   `σ² = (1/n)·Σ[ln(H/C)·ln(H/O) + ln(L/C)·ln(L/O)]` is **unbiased
   under non-zero drift**. When a series has material trend,
   Parkinson/GKVOL will under- or over-estimate variance; RSVOL
   remains correct. Having all three lets an agent cross-check:
   large disagreement between GKVOL and RSVOL ⇒ significant drift
   in the window.

4. **No coherent tail-loss measure.** TAILR uses the quantile ratio
   `|pct_95| / |pct_05|` — shape metric. DOWNVOL uses the variance
   of negative returns — scale metric. But neither answers the
   canonical risk question: "given we are in the worst 5% of days,
   what is the *average* loss?" That is Expected Shortfall /
   Conditional VaR (CVaR), the coherent downside-risk measure
   preferred by Basel III and modern risk frameworks (it satisfies
   subadditivity, which plain VaR does not).

5. **No weekday calendar axis.** MONTHSEAS (Round 27) captures
   monthly seasonality. But a second canonical calendar axis is
   day-of-week: Monday-effect, Friday-rally, Wednesday-weakness.
   Long-horizon academic literature (French 1980; Ariel 1987; many
   replications) shows consistent weekday patterns that only a
   DOW lens can see. A weekday hit-rate snapshot completes the
   calendar pair.

Round 28 ships these five surfaces as ADR-136. Same additive
envelope as Rounds 5–27: no new fetchers, no cross-symbol scans,
no new external API dependencies. PARKINSON/GKVOL/RSVOL/CVAR
compute from the trailing 253-session window on the existing HP
cache. DOWEFFECT uses the *full* HP cache (not windowed), same as
MONTHSEAS, because weekday effect significance requires years of
intraday O→C history.

## Decision

Ship Round 28 as a five-surface additive bundle using schema v29
layered on v28:

| Surface    | Table                 | Purpose                                                        |
|------------|-----------------------|----------------------------------------------------------------|
| PARKINSON  | `research_parkinson`  | Parkinson (1980) H-L range vol (5.2× efficiency)                |
| GKVOL      | `research_gkvol`      | Garman-Klass (1980) OHLC vol (7.4× efficiency)                  |
| RSVOL      | `research_rsvol`      | Rogers-Satchell (1991) drift-independent OHLC vol               |
| CVAR       | `research_cvar`       | Conditional VaR / Expected Shortfall at 5% and 1%               |
| DOWEFFECT  | `research_doweffect`  | Day-of-week intraday (O→C) seasonality hit rate + mean return   |

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

- **PARKINSON / GKVOL / RSVOL**: `VERY_LOW` (<10%) / `LOW` (<20%) /
  `NORMAL` (<40%) / `HIGH` (<60%) / `VERY_HIGH` (≥60%) — all buckets
  read against the *annualized* σ in %.
- **CVAR**: `MINIMAL` (|ES(5%)|<1%) / `LOW` (<2.5%) / `MODERATE`
  (<5%) / `HIGH` (<10%) / `EXTREME` (≥10%).
- **DOWEFFECT**: `STRONG_EFFECT` (best-worst hit spread ≥20%) /
  `MILD_EFFECT` (≥10%) / `NEUTRAL` (≥5%) / `INCONSISTENT` (<5%).

## Consequences

### Positive

- **Full OHLC-vol family complete.** PARKINSON (H-L range only) →
  GKVOL (H-L + C-O, efficiency-optimal at zero drift) → RSVOL
  (drift-independent). Agents can now compare the three and infer
  both the volatility level and whether significant drift is
  present in the window.
- **Coherent tail-risk measure added (CVAR).** First packet
  surface that reports Expected Shortfall — the modern risk
  industry's preferred downside measure, distinct from TAILR
  (shape ratio) and DOWNVOL (variance of negative returns).
- **Calendar axis pair complete.** MONTHSEAS (Round 27) covered
  monthly; DOWEFFECT covers weekday. Together they capture both
  canonical calendar axes of academic and practitioner finance
  seasonality research.
- **Drift detection as a side-effect.** A material gap between
  GKVOL annualized σ and RSVOL annualized σ directly implies the
  251-day window had non-zero drift significant enough to bias the
  zero-drift estimators — a free diagnostic.

### Negative / Risks

- **Schema migration.** `create_research_tables_v29` is additive
  over v28, so peers on v28 who receive v29 rows through LAN sync
  will create the 5 new tables via the existing create-before-insert
  path. No back-compat break.
- **DOWEFFECT is full-HP-scan.** Same O-profile consideration as
  MONTHSEAS: O(N) over full HP cache rather than O(253). Still
  cheap but different from PARKINSON/GKVOL/RSVOL/CVAR.
- **CVAR requires ≥100 bars** (same gate as DFA). 253 bars ≫ 100,
  so the HP-local trailing window is fine; but young series with
  <100 bars return `INSUFFICIENT_DATA`.
- **DOWEFFECT needs ≥10 samples per weekday.** Holidays cluster
  asymmetrically (Mondays lose more samples to Memorial Day,
  Presidents' Day, Labor Day, MLK; Fridays lose Good Friday,
  Black Friday half-sessions). The ≥10-per-weekday gate keeps
  statistics honest; a very young ticker may pass the 100-bar
  overall gate but fail per-weekday and get `INSUFFICIENT_DATA`.
- **Packet weight.** Each surface adds ~300-700 bytes per symbol.
  Updated envelope numbers appear in the RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention
  established in Rounds 24–27 (UP=green, DOWN=red,
  AXIS_TEXT=neutral).
- **All five surfaces use the same broker handler shape** that has
  been stable since Round 22: `BrokerCmd::Compute*Snapshot →
  tokio::spawn → compute → msg_tx.send(BrokerMsg::*SnapshotMsg)`.
  The receive arm in `update()` both updates UI state and upserts
  to SQLite. LAN sync fans out on the next window.
- **Palette aliases** avoid prior bindings. `OHLC_VOL` is already
  owned by VOLE (Yang-Zhang), so GKVOL uses `GARMAN_KLASS_VOL`
  instead. `SEASONALITY` is owned by SEAG (Round 13) and MONTHSEAS
  (Round 27) keeps the month-specific aliases. DOWEFFECT uses
  `DOW_EFFECT` / `DOW` / `WEEKDAY_EFFECT` / `DAY_OF_WEEK`.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 398
  passing (up from 383 in Round 27, +15 new: 5 roundtrip + 10
  compute tests).
- `cargo check -p typhoon-native` — clean after palette collision
  fix (`OHLC_VOL` → `GARMAN_KLASS_VOL` in GKVOL aliases).
- Each new compute surface carries at least one "rising OHLC",
  "tailed", or "dated dow-pattern" deterministic-fixture compute
  test that validates the label path beyond just the
  `INSUFFICIENT_DATA` sentinel. The DOWEFFECT test walks real
  weekdays starting from 2022-01-03 (Monday) and injects a
  Friday-rally + Monday-slump pattern to verify the best=Fri,
  worst=Mon ranking is recovered.

## Packet envelope

After Round 28, single-symbol packet target envelope is **~52-102 KB**
(up from 50-98 in Round 27). Basket (10 symbols via BASKET) is
**~510-1020 KB** (up from 490-980). Sub-block count grows 128 → 133.

Total HP-local research snapshot count after Round 28: **92**
(87 + 5). Total cross-symbol rank snapshots unchanged.
