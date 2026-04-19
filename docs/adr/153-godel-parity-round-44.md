# ADR-153: Godel Parity Round 44 — ADX / CCI / CMF / MFI / PSAR

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-152
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| ADX | Canonical (all terminals) | Yes (`ADX` / `DX` / `PLUS_DI` / `MINUS_DI`) | Yes | Yes | No (deferred — ADR-188) |
| CCI | Canonical (all terminals) | Yes (`CCI`) | Yes | Yes | No (deferred — ADR-188) |
| CMF | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| MFI | Canonical (all terminals) | Yes (`MFI`) | Yes | Yes | No (deferred — ADR-188) |
| PSAR | Canonical (all terminals) | Yes (`SAR` / `SAREXT`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** canonical technical-analysis primitives common across all terminals (Wilder ADX via TA-Lib `ADX`, Lambert CCI via `CCI`, Chaikin Money Flow, Money Flow Index via `MFI`, Wilder Parabolic SAR via `SAR`). All except CMF are TA-Lib primitives.

## Context

Round 43 (ADR-152) shipped ICHIMOKU/SUPERTREND/KELTNER/FISHER/AROON,
taking HP-local research surfaces to 167 and per-symbol sub-blocks to
208. Continuing the "combing over for full Godel research parity"
directive, Round 44 closes five more classical technical-analysis
gaps that remained after Round 43.

1. **No Wilder ADX / directional movement system.** ADX (Wilder,
   *New Concepts in Technical Trading Systems*, 1978) is the
   canonical "trend *strength*" oscillator. +DM = max(H−H_prev, 0),
   −DM = max(L_prev−L, 0); whichever is greater wins the bar.
   Smoothed with Wilder's 14-period averaging, divided by ATR
   to give +DI / −DI, then DX = 100·|+DI − −DI|/(+DI + −DI);
   ADX is Wilder-smoothed DX. Header gives **adx_label**
   (STRONG_TREND adx≥40 / TREND ≥25 / WEAK_TREND ≥15 / NO_TREND /
   INSUFFICIENT_DATA). Complements AROON (Round 43): Aroon measures
   *time-since-extreme*, ADX measures *strength regardless of time*.

2. **No Lambert CCI.** The Commodity Channel Index (Donald Lambert,
   *Commodities*, 1980) is period=20 by default. TP = (H+L+C)/3,
   MAD = mean(|TP − SMA(TP, 20)|), CCI = (TP − SMA) / (0.015·MAD).
   The 0.015 scaling was chosen by Lambert so that ~70–80% of
   values fall in [−100, +100]. Header gives **cci_label**
   (OVERBOUGHT >100 / BULL >0 / NEUTRAL / BEAR <0 / OVERSOLD <−100
   / INSUFFICIENT_DATA). Distinct from RSI: CCI is *mean-deviation*
   normalised, not gain/loss-ratio — so a strong trend of one-sided
   moves produces different extremes than RSI.

3. **No Chaikin Money Flow.** CMF (Marc Chaikin, 1980s) is a
   volume-weighted accumulation/distribution oscillator.
   MFV = ((C − L) − (H − C)) / (H − L) × volume (the "money flow
   volume" per bar); CMF = Σ MFV / Σ volume over 20 bars. Output
   bounded in [−1, +1]. Header gives **cmf_label** (STRONG_ACCUM
   >0.25 / ACCUM >0.05 / NEUTRAL / DIST <−0.05 / STRONG_DIST
   <−0.25 / INSUFFICIENT_DATA). First volume-weighted
   accumulation-line surface we've shipped — complements OBV-style
   momentum but normalised.

4. **No Money Flow Index.** MFI (Quong & Soudack, 1989) is a
   volume-weighted RSI over 14 bars. Typical-price × volume =
   "raw money flow"; bar classified as positive/negative by
   direction of TP change; ratio = Σpos / Σneg; MFI = 100 − 100/(1+ratio).
   Header gives **mfi_label** (OVERBOUGHT >80 / BULL >50 / NEUTRAL /
   BEAR <50 / OVERSOLD <20 / INSUFFICIENT_DATA). Known as
   "volume-weighted RSI" — differs from RSI in that bars with
   heavy volume count more toward the oscillator.

5. **No Wilder PSAR.** The Parabolic Stop-And-Reverse (Wilder, 1978)
   is a trailing-stop indicator that accelerates in the direction
   of the trend. Initial AF = 0.02, increment 0.02 each time a new
   extreme point (EP) is made, capped at 0.20. SAR_next = SAR +
   AF·(EP − SAR); flips when price crosses SAR, with the new SAR
   clamped to the prior-two-bar low (long-to-short flip) or high
   (short-to-long flip). Header gives **psar_label** (STRONG_UP /
   UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA). Complements
   SUPERTREND (Round 43): PSAR accelerates (AF grows), SuperTrend
   is ATR-proportional and does not accelerate — so PSAR fires
   trailing-stop exits earlier in strong trends.

Round 44 ships these five surfaces as ADR-153. Same additive envelope
as Rounds 5–43: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing HP
cache.

## Decision

Ship Round 44 as a five-surface additive bundle using schema v45
layered on v44:

| Surface | Table                 | Purpose                                                                   |
|---------|-----------------------|---------------------------------------------------------------------------|
| ADX     | `research_adx`        | Wilder's Directional Movement Index (+DI, −DI, ADX, DX, ATR)              |
| CCI     | `research_cci`        | Lambert's Commodity Channel Index (TP, SMA, MAD, CCI)                     |
| CMF     | `research_cmf`        | Chaikin Money Flow (volume-weighted accumulation/distribution)            |
| MFI     | `research_mfi`        | Money Flow Index (volume-weighted RSI)                                    |
| PSAR    | `research_psar`       | Wilder's Parabolic Stop-And-Reverse (AF 0.02/0.02/0.20)                   |

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
`INSUFFICIENT_DATA` sentinel). Label thresholds summarised above.

## Consequences

### Positive

- **Closes the Wilder-suite directional gaps.** Wilder's 1978 book
  introduced RSI (already shipped), ATR (already shipped), and ADX +
  PSAR — Round 44 completes the set. ADX and PSAR together are the
  two Wilder indicators that specifically measure trend structure
  (strength and trailing-stop direction) rather than oscillator
  levels.
- **Adds two volume-weighted surfaces (CMF + MFI).** Previous
  rounds were price-only or ATR-weighted; CMF is the first
  accumulation-line surface bounded in [−1, +1] and MFI is the
  first volume-weighted RSI. These are distinct signals from any
  price-only oscillator because they incorporate the volume
  confirmation dimension.
- **CCI fills the mean-deviation gap.** RSI/Stoch/Williams%R all
  use gain/loss or high-low ratios; CCI uses mean-absolute-deviation
  normalisation. A tape with many small moves in one direction
  produces different CCI extremes than RSI, so CCI often fires
  earlier on long slow grinds.
- **No new external dependencies, no fetcher expansion.** Pure
  compute on the HP cache — same additive envelope as Rounds 26–43.

### Negative / Risks

- **Schema migration.** `create_research_tables_v45` is additive
  over v44; peers on v44 who receive v45 rows via LAN sync will
  create the 5 new tables via the existing create-before-insert
  path. No back-compat break.
- **ADX warmup is long.** Wilder smoothing needs 2·period = 28
  bars of seed data before ADX stabilises; we require 30+ bars
  minimum and label shorter tapes as `INSUFFICIENT_DATA`.
  Documented.
- **PSAR is path-dependent.** Like SuperTrend (Round 43), the
  flip recursion compares current price to the *previous* SAR
  value; re-computing on a rolling window produces slightly
  different band values than computing on a fixed tail window.
  We compute on the full available HP cache so the answer is
  deterministic *given the cache state* but peers with different
  cache depths may see different flip points in early bars.
  Documented as "evaluate on current cache" rather than "rolling
  backtest".
- **CMF and MFI require volume.** Unlike every prior price-only
  surface, CMF and MFI require the `volume` column on the HP
  cache. Bars with zero volume (holidays, halts) are skipped in
  the MFV / money-flow sums rather than treated as zero-flow;
  documented as a deliberate choice to avoid zero-volume bars
  pinning CMF toward zero.
- **Bar with H == L on CMF.** The MFV formula divides by (H − L);
  we guard with an epsilon and emit MFV = 0 on flat bars. Without
  the guard, flat doji bars would NaN the entire CMF sum.
  Documented.
- **Packet weight.** ADX adds ~240 bytes, CCI ~210, CMF ~210,
  MFI ~220, PSAR ~270. Total Round 44 addition: ~1.15 KB/symbol.
  Updated envelope numbers appear in the RESEARCH_PACKET.md
  header.

### Neutral

- **Label-based color scheme continues** the convention from
  Rounds 24–43. For CCI/MFI, OVERBOUGHT uses the *warning* color
  (red) since it flags a mean-reversion setup at the top —
  consistent with the "label is a signal, not a direction" rule.
- **Palette alias disambiguation.** Bare `ADX`, `CCI`, `PSAR` are
  already bound to chart-overlay toggles upstream (chart-overlay
  booleans `show_adx`, `show_cci`, `show_psar`). Round 44 research
  windows use disambiguated aliases only (e.g. `ADXFIT`, `ADXWIN`,
  `CCIFIT`, `CCIWIN`, `PSARFIT`, `PSARWIN`) to avoid shadowing the
  chart-overlay handlers. `CMF` and `MFI` bare names are unbound
  (verified via grep across `native/src/app.rs`) and kept as
  aliases for the research windows.
- **All five surfaces use the same broker handler shape** stable
  since Round 22. All compute purely from the HP cache — no
  cross-symbol reads.
- **Field-name `_win` suffix** on native struct fields follows the
  Round 43 convention (`adx_win_symbol`, `adx_win_snapshot`,
  `adx_win_open`, `adx_win_loading`, `adx_win_date`) to avoid
  colliding with chart-overlay booleans in the same `TyphoonApp`
  struct.

### Paid-API gap (for later revisit)

Same as ADR-152. The remaining gaps are data-access-gated
(intraday bars, Level-2 order book depth, options IV surfaces,
corporate actions feeds, realised-variance matrices, insider
transactions feed). No Round 44 surface needed any of these; all
compute from the daily HP cache. These will be revisited when/if
paid API access is sanctioned.

## Verification

- `cargo test -p typhoon-engine --lib` — 1156 passing (up from
  1146 in Round 43, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; Round 44 field names use
  `_win` suffix to avoid collision with existing chart-overlay
  booleans (`show_adx`, `show_cci`, `show_psar`).
- ADX/CCI/CMF/MFI/PSAR compute_oscillating use the ±0.5%
  oscillating fixture (150 bars). Each asserts label belongs to
  its regime set, scalars are finite when label is not
  INSUFFICIENT_DATA, and axis-specific invariants:
  ADX +DI and −DI ∈ [0, 100], ADX and DX ∈ [0, 100], ATR finite
  and positive; CCI value finite, SMA and MAD positive; CMF
  value ∈ [−1, 1], volume sum positive; MFI value ∈ [0, 100],
  money-flow ratio finite and non-negative; PSAR sar_value
  finite, acceleration_factor ∈ [0.02, 0.20], bars_in_trend ≥ 0,
  trend_is_up a bool.

## Packet envelope

After Round 44, single-symbol packet target envelope is **~70-139 KB**
(up from 69-137 in Round 43). Basket (10 symbols via BASKET) is
**~700-1390 KB** (up from 690-1370). Sub-block count grows 208 → 213.

Total HP-local research snapshot count after Round 44: **172**
(167 + 5). Total cross-symbol rank snapshots unchanged.
