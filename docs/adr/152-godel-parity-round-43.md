# ADR-152: Godel Parity Round 43 — ICHIMOKU / SUPERTREND / KELTNER / FISHER / AROON

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-151
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 42 (ADR-151) shipped SQUEEZE/SQUEEZERANK/BBSQUEEZE/DONCHIAN/KAMA,
taking HP-local research surfaces to 162 and per-symbol sub-blocks to
203. Continuing the "combing over for full Godel research parity"
directive, Round 43 closes five more orthogonal gaps in the classical
technical-analysis library that were absent from the existing suite.

1. **No Ichimoku Kinkō Hyō cloud.** The Ichimoku system is the
   canonical "one-glance equilibrium chart" from Japanese technical
   analysis — five overlay lines (Tenkan-sen, Kijun-sen, Senkou Span
   A, Senkou Span B, Chikou Span) that together identify trend,
   support/resistance, and momentum in a single visualization. We
   compute Tenkan(9) = (maxH9 + minL9)/2, Kijun(26) = (maxH26 +
   minL26)/2, Senkou A = (Tenkan + Kijun)/2, Senkou B(52) = (maxH52 +
   minL52)/2, Chikou = close shifted back 26 bars. Header gives
   **ichimoku_label** (STRONG_BULL / BULL / NEUTRAL / BEAR /
   STRONG_BEAR / INSUFFICIENT_DATA) driven by close position relative
   to the cloud (above Senkou A + B → bull) and T/K cross direction.

2. **No SuperTrend ATR-channel overlay.** SuperTrend is a Wilder-ATR
   trailing-stop indicator (period=10, multiplier=3) with a strict
   flip recursion: upper = hl2 + m·ATR, lower = hl2 − m·ATR; the
   active band only tightens in the trend direction until the close
   crosses it, at which point it flips. The closed-loop band serves
   as a dynamic support/resistance line. Complements DONCHIAN's pure
   N-bar breakout: SuperTrend is regime-aware (tracks volatility),
   DONCHIAN is event-based (20-bar envelope).

3. **No Keltner Channel.** Keltner uses an EMA midline ± N·ATR bands
   (EMA=20, ATR=10, multiplier=2). Critically, pairs with BBSQUEEZE
   (ADR-151) to produce the **TTM-Squeeze** signal: when the
   Bollinger Bands are fully *inside* the Keltner Channel
   (BB_upper ≤ KC_upper AND BB_lower ≥ KC_lower), volatility is
   compressed — a canonical breakout precursor identified by John
   Carter (*Mastering the Trade*, 2005). Header gives
   **keltner_label** (STRONG_BULL above upper / BULL / NEUTRAL /
   BEAR / STRONG_BEAR below lower) plus a separate `ttm_squeeze`
   boolean field.

4. **No Ehlers Fisher Transform.** The Fisher Transform (John Ehlers,
   2002) applies `0.5 · ln((1+x)/(1−x))` to a normalised price series
   to produce a distribution with sharper peaks than raw returns —
   making turning-point detection faster at the cost of higher
   false-positive rate on strong trends. We use hl2 midline rescaled
   to [−0.999, 0.999] over a 10-bar window with 0.66/0.67 smoothing
   weights (Ehlers' canonical values), 0.5 prior feedback. Header
   gives **fisher_label** (PEAK_HIGH / BULL / NEUTRAL / BEAR /
   PEAK_LOW) — PEAK labels flag the saturated regions where the
   transform is about to revert. Complementary oscillator to TSI
   (ADR-146) and RSI-style measures.

5. **No Aroon oscillator.** Aroon (Chande, 1995) measures time since
   the highest high and lowest low over a 25-bar window:
   Aroon_Up = 100 · (period − bars_since_high) / period,
   Aroon_Down = 100 · (period − bars_since_low) / period,
   Oscillator = Up − Down ∈ [−100, +100]. Header gives
   **aroon_label** (STRONG_UP osc > 50 / WEAK_UP > 25 /
   CONSOLIDATION / WEAK_DOWN < −25 / STRONG_DOWN < −50). Unlike
   ADX/CHOP (trend *strength*), Aroon measures *time-since-extreme*
   — a distinct construct that fires early in new trends when the
   highest-high just printed.

Round 43 ships these five surfaces as ADR-152. Same additive envelope
as Rounds 5–42: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing HP
cache.

## Decision

Ship Round 43 as a five-surface additive bundle using schema v44
layered on v43:

| Surface     | Table                  | Purpose                                                                 |
|-------------|------------------------|-------------------------------------------------------------------------|
| ICHIMOKU    | `research_ichimoku`    | Ichimoku Kinkō Hyō cloud (Tenkan/Kijun/Senkou A/B/Chikou)               |
| SUPERTREND  | `research_supertrend`  | Wilder-ATR SuperTrend trailing-stop band with flip detection            |
| KELTNER     | `research_keltner`     | Keltner Channel (EMA ± N·ATR) + TTM-Squeeze flag via BB pairing         |
| FISHER      | `research_fisher`      | Ehlers Fisher Transform on hl2 midline                                  |
| AROON       | `research_aroon`       | Aroon Up/Down/Oscillator time-since-extreme oscillator                  |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (5 active buckets +
`INSUFFICIENT_DATA` sentinel). Label thresholds summarised above.

## Consequences

### Positive

- **Covers the "classical Japanese + trailing-stop + channel" axis.**
  Ichimoku is the most widely-taught single-glance trend system and
  was absent; SuperTrend is the modern trailing-stop equivalent of the
  parabolic SAR and fills the regime-aware trailing-stop gap.
- **Completes the TTM-Squeeze pairing.** BBSQUEEZE (ADR-151) ranks
  Bollinger-band width against its own history; Keltner is the other
  half of the canonical TTM construct (John Carter). Together they
  fire the `ttm_squeeze` flag: BB fully inside KC → volatility
  compression → breakout precursor. This is a distinct signal from
  either in isolation.
- **First Ehlers-style transform.** The Fisher Transform's Gaussian-
  symmetric distribution lets the PEAK_HIGH / PEAK_LOW labels flag
  *saturated* regions where the oscillator is about to revert — a
  different construct from TSI/RSI threshold-crossings.
- **Aroon complements ADX/CHOP.** ADX measures trend *strength*
  regardless of time; Aroon measures *time since the extreme*. Fires
  earlier in fresh trends (the moment a new 25-bar high prints,
  Aroon_Up jumps to 100 even if ADX is still building).
- **No new external dependencies, no fetcher expansion.** Pure
  compute on the HP cache — same additive envelope as Rounds 26–42.

### Negative / Risks

- **Schema migration.** `create_research_tables_v44` is additive over
  v43; peers on v43 who receive v44 rows via LAN sync will create the
  5 new tables via the existing create-before-insert path. No
  back-compat break.
- **Ichimoku lookback is long.** Senkou B needs 52 bars and Chikou
  shifts 26 bars back, so the minimum usable window is 78 bars. This
  is longer than most other surfaces; symbols with short histories
  emit `INSUFFICIENT_DATA`. Documented.
- **SuperTrend is path-dependent.** The flip recursion compares
  current close to the *previous* band value; this means
  re-computing SuperTrend on a rolling window produces slightly
  different band values than computing on a fixed tail window. We
  compute on the full available HP cache (up to 253 bars) so the
  answer is deterministic *given the cache state* — but peers with
  different cache depths may see different flip points in the early
  bars. Documented as "evaluate on current cache" rather than
  "rolling backtest".
- **Keltner vs BBSQUEEZE overlap.** Both measure compression; BB
  (std-dev-based) reacts faster to quiet periods, KC (ATR-based)
  reacts slower. The TTM-Squeeze flag specifically requires *both*
  conditions to hold — a useful stricter filter than either alone.
- **Fisher Transform saturation.** The atanh-like transform explodes
  as x → ±1. We clamp the normalised input to [−0.999, 0.999] to
  bound the output; Ehlers' original doesn't clamp and can produce
  infinities on flat tape. Documented as a deliberate defensive
  choice.
- **Aroon at period=25 is the canonical default** but is short for
  daily charts. Larger periods (50, 100) would be a configurability
  item for later; fixed at 25 for parity with TradingView/Godel
  defaults.
- **Packet weight.** ICHIMOKU adds ~300 bytes, SUPERTREND ~230,
  KELTNER ~260 (includes inline BB bands for TTM flag), FISHER ~190,
  AROON ~210. Total Round 43 addition: ~1.2 KB/symbol. Updated
  envelope numbers appear in the RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established
  in Rounds 24–42 (UP=green for "favorable" label, DOWN=red for
  "adverse", AXIS_TEXT=neutral). For Fisher, PEAK_HIGH uses the
  *warning* color (red) since it flags a mean-reversion setup at the
  top — counterintuitive but consistent with the "label is a signal,
  not a direction" rule.
- **Palette alias disambiguation.** Bare `ICHIMOKU`, `SUPERTREND`,
  `KELTNER`, `FISHER` are already bound to chart-overlay toggles
  upstream (indicator plots on the main chart pane). Round 43
  research windows use disambiguated aliases only (e.g.
  `ICHIMOKUFIT`, `ICHIMOKUWIN`, `SUPERTRENDWIN`, `KELTNERFIT`,
  `FISHERFIT`, etc.) to avoid shadowing the chart-overlay handlers.
  `AROON` bare is unbound (verified via grep across
  `native/src/app.rs`) and kept as an alias for the research window.
- **All five surfaces use the same broker handler shape** that has
  been stable since Round 22. All compute purely from the HP cache
  — no cross-symbol reads.

### Paid-API gap (for later revisit)

Same as ADR-151. The gaps remain data-access-gated (intraday bars,
order-book depth, options IV surfaces, corporate actions feeds,
realised-variance matrices). No Round 43 surface needed any of these;
all compute from the daily HP cache.

## Verification

- `cargo test -p typhoon-engine --lib` — 1146 passing (up from 1136
  in Round 42, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; Round 43 field names use
  `_win` suffix to avoid collision with existing chart-overlay
  booleans (`show_ichimoku`, `show_supertrend`, `show_keltner`,
  `show_fisher`).
- ICHIMOKU/SUPERTREND/KELTNER/FISHER/AROON compute_oscillating use
  the ±0.5% oscillating fixture (150 bars). Each asserts label
  belongs to its regime set, scalars are finite when label is not
  INSUFFICIENT_DATA, and axis-specific invariants:
  ICHIMOKU Tenkan/Kijun/SenkouA/SenkouB all finite and within the
  (min_low, max_high) bounding box; SUPERTREND upper ≥ lower, band
  value finite, direction ∈ {1, -1}; KELTNER upper ≥ mid ≥ lower,
  bb_upper ≥ bb_lower, ttm_squeeze a bool; FISHER value and trigger
  ∈ [−5, 5] post-clamp; AROON up/down ∈ [0, 100], oscillator ∈
  [−100, 100], period = 25.

## Packet envelope

After Round 43, single-symbol packet target envelope is **~69-137 KB**
(up from 68-135 in Round 42). Basket (10 symbols via BASKET) is
**~690-1370 KB** (up from 680-1350). Sub-block count grows 203 → 208.

Total HP-local research snapshot count after Round 43: **167**
(162 + 5). Total cross-symbol rank snapshots unchanged.
