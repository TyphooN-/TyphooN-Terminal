# ADR-149: Godel Parity Round 40 — DURBINWATSON / BDSTEST / BREUSCHPAGAN / TURNPTS / PERIODOGRAM

**Status:** Accepted
**Date:** 2026-04-16
**Supersedes/extends:** ADR-108 through ADR-148
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| DURBINWATSON | No | No | Yes | Yes | No (deferred — ADR-188) |
| BDSTEST | No | No | Yes | Yes | No (deferred — ADR-188) |
| BREUSCHPAGAN | No | No | Yes | Yes | No (deferred — ADR-188) |
| TURNPTS | No | No | Yes | Yes | No (deferred — ADR-188) |
| PERIODOGRAM | No | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure quant/statistical econometric primitives (Durbin-Watson AR(1) residual test, Brock-Dechert-Scheinkman nonlinear-iid test, Breusch-Pagan heteroskedasticity LM, Bartels turning-points test, Schuster periodogram) — not documented Godel Terminal features and not TA-Lib catalog entries; classical quant-literature stats.

## Context

Round 39 (ADR-147) shipped GARCH11/SADF/CORDIM/SKSPEC/AUTOMI, pushing
HP-local research surfaces to 147 and per-symbol sub-blocks to 188. Five
more canonical surfaces remain, each on an axis of classical
econometrics that the packet has not yet surfaced:

1. **No residual-autocorrelation diagnostic.** LJUNGB (ADR-137) tests
   block-lag serial correlation. The classical Durbin-Watson d-statistic
   answers the narrower question that practitioners see every day on a
   regression printout — "is there first-order AR(1) correlation in the
   residuals?". d∈[0,4] with d≈2 signalling independence, d<1 strong
   positive, d>3 strong negative. DURBINWATSON ships d, the implied ρ̂
   estimate ρ̂ ≈ 1 − d/2, and a five-level label.

2. **No iid / nonlinear-independence test.** LJUNGB catches linear
   dependence; AUTOMI (Round 39) catches *any* pairwise information at a
   specific lag. Neither directly tests the joint-iid null over an
   embedding — the Brock-Dechert-Scheinkman (1987/1996) BDS test does.
   BDS compares the correlation integral C_m(ε) against C_1(ε)^m under
   the iid null and reports an asymptotically-standard-normal statistic.
   A significant BDS is the canonical signature of hidden nonlinear
   structure (ARCH effects, regime-switching, chaos).

3. **No heteroskedasticity LM test.** ARCHLM (ADR-139) tests
   autoregressive conditional heteroskedasticity. The classical
   Breusch-Pagan (1979) test asks a different question: does residual
   variance depend on an explanatory variable? We use the bar index as
   the sole regressor — a minimal "is variance trending over the window?"
   check. LM = n×R² on the auxiliary regression of squared residuals,
   compared to χ²(1). Complements ARCHLM by testing long-run rather than
   autoregressive heteroskedasticity.

4. **No non-parametric randomness test.** RUNSTEST (ADR-137) counts
   runs above/below the median. The Bartels / turning-points test counts
   strict local extrema. Under iid the expected count is 2(n−2)/3 with
   variance (16n−29)/90 — a parameter-free z-statistic. Significantly
   over-turning suggests reversal / whipsaw behaviour; under-turning
   suggests drift / trend regimes. Useful as a sanity check on other
   serial-independence tests.

5. **No spectral / dominant-cycle diagnostic.** DFA (ADR-128), HIGUCHI
   (ADR-145), MFDFA (ADR-146), and the auto-correlation family all
   probe the series in the time domain. Nothing yet answers the
   classical question "is there a dominant cycle period?". The
   Schuster (1898) periodogram computed via direct DFT answers this
   directly: report the frequency with peak spectral power, the
   corresponding period in bars, the dominant-to-total power ratio, and
   a four-level cycle-strength label. Mostly for triangulating
   MFDFA / AUTOMI findings against conventional Fourier analysis.

Round 40 ships these five surfaces as ADR-149. Same additive envelope
as Rounds 5–39: no new fetchers, no cross-symbol scans, no new external
API dependencies. All five compute from the trailing 253-session window
on the existing HP cache.

## Decision

Ship Round 40 as a five-surface additive bundle using schema v41
layered on v40:

| Surface        | Table                      | Purpose                                                             |
|----------------|----------------------------|---------------------------------------------------------------------|
| DURBINWATSON   | `research_durbinwatson`    | Durbin-Watson d-statistic / first-order residual autocorrelation    |
| BDSTEST        | `research_bdstest`         | Brock-Dechert-Scheinkman iid / nonlinear-independence test           |
| BREUSCHPAGAN   | `research_breuschpagan`    | Breusch-Pagan LM test for trending residual variance                |
| TURNPTS        | `research_turnpts`         | Bartels turning-points test for non-randomness                      |
| PERIODOGRAM    | `research_periodogram`     | Direct-DFT periodogram / dominant-cycle detection                   |

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
`INSUFFICIENT_DATA` sentinel). Label strings:

- **DURBINWATSON**: `STRONG_POS` (d<1) / `WEAK_POS` (<1.5) /
  `NO_AUTOCORR` (1.5–2.5) / `WEAK_NEG` (<3.0) / `STRONG_NEG`
  (otherwise).
- **BDSTEST**: `IID_CONFIRMED` (p≥0.05) / `WEAK_DEPENDENCE`
  (|BDS|<4) / `STRONG_DEPENDENCE` (otherwise).
- **BREUSCHPAGAN**: `HOMOSKEDASTIC` (LM≤χ²_95=3.841) / `MILD_HETERO`
  (<10.83) / `STRONG_HETERO` (otherwise).
- **TURNPTS**: `RANDOM_IID` (p≥0.05) / `OVER_TURNING` (z>0) /
  `UNDER_TURNING` (z<0).
- **PERIODOGRAM**: `STRONG_CYCLE` (ratio>0.25) / `MODERATE_CYCLE`
  (>0.12) / `WEAK_CYCLE` (>0.05) / `NO_CYCLE` (otherwise).

## Consequences

### Positive

- **Covers the "classical regression diagnostics" axis** missing since
  round 1. DURBINWATSON and BREUSCHPAGAN are the two most-reported
  regression-table diagnostics in academic finance — their absence from
  the packet has been a small-but-noticeable gap relative to Bloomberg-
  style printouts.
- **BDS gives a joint-embedding iid test** that complements AUTOMI and
  LJUNGB. Where AUTOMI shows pairwise MI at specific lags and LJUNGB
  sums linear ACFs, BDS tests the m-dimensional embedding as a whole —
  the most-cited nonlinear-independence test in applied econometrics.
- **TURNPTS adds a parameter-free runs-style check** orthogonal to
  RUNSTEST. Where RUNSTEST counts sign-relative-to-median, TURNPTS
  counts strict local extrema — the two tests catch different failure
  modes of the iid null.
- **First Fourier-domain surface.** All prior spectral analysis has
  been time-domain (DFA, MFDFA, etc.). PERIODOGRAM ships the direct DFT
  peak + ratio, a reference diagnostic even when its spectral leakage
  limits are well-known — mostly for triangulation against the
  multi-fractal family.
- **No new external dependencies, no fetcher expansion.** Pure
  econometric compute on the HP cache — the same additive envelope as
  Rounds 26–39.

### Negative / Risks

- **Schema migration.** `create_research_tables_v41` is additive over
  v40, so peers on v40 who receive v41 rows via LAN sync will create
  the 5 new tables via the existing create-before-insert path. No
  back-compat break.
- **BDS variance approximation.** Full Brock (1996) asymptotic variance
  requires the K-statistic (C_1 correlation integral with doubled
  radius). We use a simplified σ²_m ≈ 4·c1^{2m}·(1−c1^{2m})·m which is
  a tractable upper bound — conservative (biases toward non-rejection
  at marginal p-values). For sharp p-values the user should cross-check
  against a dedicated econometrics package (statsmodels, R `tseries`).
  Acceptable tradeoff: the label still distinguishes IID from nonlinear
  dependence, which is the practical question. Documented here and in
  the struct's `note` field.
- **Periodogram spectral leakage.** Direct DFT without windowing
  (Hamming / Hann / Welch) leaks power from spectral peaks into nearby
  bins. For the "is there a dominant cycle?" question this matters less
  than exact peak positioning — we report the raw-periodogram peak.
  True spectral-density estimation would use a multitaper or Welch
  method; noted for future refinement.
- **BREUSCHPAGAN regressor choice.** We use the bar index as the sole
  regressor — a minimal test for "is variance trending over time?". A
  full BP test would regress on all candidate explanatory variables, but
  the packet has no canonical regressor set. Documented in the label
  thresholds; the test catches monotonic trends in variance and is a
  reasonable complement to ARCHLM's autoregressive test.
- **Periodogram cost.** Direct DFT is O(n × n/2). For n=253 that's
  ~16k FMA ops per symbol. Negligible in absolute terms but still a
  step up from the constant-time KURTOSIS/SKEWNESS surfaces. FFT would
  be O(n log n) but adds a dependency we don't currently carry for
  other compute.
- **Packet weight.** DW adds ~120 bytes, BDS ~240, BP ~220, TURNPTS
  ~240, PERIODOGRAM ~280. Total Round 40 addition: ~1.1 KB/symbol.
  Updated envelope numbers appear in the RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established in
  Rounds 24–39 (UP=green for "favorable" label, DOWN=red for "adverse",
  AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no collisions on
  `DURBINWATSON`, `BDSTEST`, `BREUSCHPAGAN`, `TURNPTS`, `PERIODOGRAM`,
  or their aliases (`DW`, `BDS`, `BP`, `BARTELS`, `PERGRAM`, etc.).
- **All five surfaces use the same broker handler shape** that has been
  stable since Round 22.

### Paid-API gap (for later revisit)

Same as ADR-147. The gaps remain data-access-gated (intraday bars,
order-book depth, options IV surfaces, corporate actions feeds,
realised-variance matrices). No Round 40 surface needed any of these;
all compute from the daily HP cache.

## Verification

- `cargo test -p typhoon-engine --lib` — 1116 passing (up from 1106 in
  Round 39, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias collisions.
- DW/BDS/BP/TURNPTS/PERIODOGRAM compute_oscillating use the ±0.5%
  oscillating fixture (150 bars, 149 log-returns). Each asserts the
  returned label belongs to its regime set, scalars are finite when
  label is not INSUFFICIENT_DATA, and axis-specific invariants:
  DW d∈[0,4]; BDS p∈[0,1], embed_dim=2; BP R²∈[0,1], critical>0;
  TURNPTS expected>0, variance>0, p∈[0,1]; PERIODOGRAM n_freqs≥1,
  dominant_power≥0, total_power>0, ratio∈[0,1].

## Packet envelope

After Round 40, single-symbol packet target envelope is **~66-132 KB**
(up from 65-131 in Round 39). Basket (10 symbols via BASKET) is
**~660-1320 KB** (up from 650-1310). Sub-block count grows 188 → 193.

Total HP-local research snapshot count after Round 40: **152**
(147 + 5). Total cross-symbol rank snapshots unchanged.
