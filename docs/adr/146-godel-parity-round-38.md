# ADR-146: Godel Parity Round 38 — BNSJUMP / PPROOT / MFDFA / HILLKS / TSI

**Status:** Accepted
**Date:** 2026-04-16
**Supersedes/extends:** ADR-108 through ADR-145
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 37 (ADR-145) shipped HIGUCHI/PICKANDS/KAPPA3/LYAPUNOV/RANKAC, pushing
HP-local research surfaces to 137 and per-symbol sub-blocks to 178 spanning
28+ orthogonal analytical dimensions. Five canonical surfaces remain, each
on an axis still missing.

1. **No formal jump-detection test.** Round 30 shipped BIPOWER (raw
   realised vs bipower variance comparison), but no Z-statistic — BIPOWER
   leaves the user to eyeball the two numbers. Barndorff-Nielsen &
   Shephard (2006) formalised the test: Z = (RV − BV) / sqrt(θ · ΣR⁴)
   where RV is the realised variance, BV = (π/2)·Σ|r_{i-1}·r_i| is the
   bipower variation, and θ = π²/4 + π − 5 is the Jarque-Bera-like
   standardisation constant. Under the null of no jumps, Z → N(0,1); a
   large positive Z rejects pure-diffusion in favour of a jump component.
   BNSJUMP ships the Z-stat, approximate p-value, jump ratio
   (RV−BV)/RV, and a four-level jump-strength label.

2. **No Phillips-Perron unit-root test.** ADF (ADR-126) and KPSS
   (ADR-144) both cover stationarity, but each has known weaknesses: ADF
   is sensitive to lag specification and low-power against near-unit
   roots, KPSS tests the reversed null. Phillips-Perron (1988) uses
   Newey-West corrections to the OLS t-statistic from a raw AR(1)
   regression — no lag specification required, robust to conditional
   heteroscedasticity. Conventional interpretation: three-way agreement
   across ADF, KPSS, and PP gives much higher confidence in the
   stationarity call than any single test. PPROOT ships ρ̂, the raw t
   statistic, PP Z(ρ) and Z(t) corrections, and the auto-picked lag
   truncation q = floor(4·(n/100)^0.25) per Schwert (1989).

3. **No multifractal spectrum.** HURST (ADR-117), DFA (ADR-130), and
   HIGUCHI (ADR-145) all ship monofractal exponents — a single number
   characterising the self-similar scaling. Real price walks exhibit
   multifractality: different moments (q-orders) give different scaling
   exponents, producing a spectrum h(q). MFDFA (Kantelhardt 2002)
   generalises DFA to arbitrary moment orders. Δh = h(−2) − h(+2)
   quantifies spectrum width: Δh ≈ 0 is monofractal, Δh > 0.3 is
   strongly multifractal (heterogeneous scaling across scales). MFDFA
   ships h(q) at q ∈ {−2, 0, +2}, Δh, and a four-level multifractality
   label.

4. **No tail-fit goodness-of-fit.** HILLTAIL (ADR-131) ships the Hill
   α̂ estimator but says nothing about whether the fitted Pareto model
   is *good*. A symbol can have a well-defined Hill α while the tail
   shape badly misfits a Pareto — log-returns with a Gumbel-shaped tail
   will produce a finite α̂ that is misleading. KS test between the
   empirical tail distribution and the fitted Pareto gives a principled
   goodness-of-fit diagnostic. HILLKS ships D = sup|F_n(x) − F_Pareto(x)|
   over the tail sample of size k, compared against the 5% critical
   value 1.36/√k. Four-level label: GOOD_FIT / ACCEPTABLE_FIT / POOR_FIT
   / REJECT.

5. **No double-smoothed momentum oscillator.** RSI (classical), MACD
   (ADR-115), and CCI (ADR-129) cover single- and dual-EMA momentum.
   William Blau's (1991) True Strength Index takes this one step further:
   TSI = 100 × EMA₁₃(EMA₂₅(ΔP)) / EMA₁₃(EMA₂₅(|ΔP|)). The double
   smoothing produces a much less-noisy momentum signal with a clear
   zero-line crossover interpretation. Unlike RSI, TSI is not range-
   bound to [0, 100] — typical values land in [−100, +100] but extreme
   regimes can exceed this. TSI ships the value, a short-EMA signal
   line, and the TSI−signal spread as a momentum-of-momentum trigger.

Round 38 ships these five surfaces as ADR-146. Same additive envelope
as Rounds 5–37: no new fetchers, no cross-symbol scans, no new external
API dependencies. All five compute from the trailing 253-session window
on the existing HP cache.

## Decision

Ship Round 38 as a five-surface additive bundle using schema v39
layered on v38:

| Surface   | Table                  | Purpose                                                             |
|-----------|------------------------|---------------------------------------------------------------------|
| BNSJUMP   | `research_bnsjump`     | Barndorff-Nielsen-Shephard 2006 jump-test Z statistic                |
| PPROOT    | `research_pproot`      | Phillips-Perron 1988 nonparametric unit-root test                   |
| MFDFA     | `research_mfdfa`       | Multifractal DFA spectrum h(q) at q ∈ {−2, 0, +2}                    |
| HILLKS    | `research_hillks`      | KS goodness-of-fit for the Hill-tail Pareto model                   |
| TSI       | `research_tsi`         | Blau 1991 True Strength Index (double-smoothed momentum oscillator) |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (4–5 active buckets +
`INSUFFICIENT_DATA` sentinel). Label strings:

- **BNSJUMP**: `STRONG_JUMP` (z>3.09) / `MODERATE_JUMP` (z>2.33) /
  `WEAK_JUMP` (z>1.65) / `NO_JUMP` (otherwise).
- **PPROOT**: `STATIONARY_STRONG` (Z(t)<−3.43) / `STATIONARY_WEAK`
  (<−2.86) / `BORDERLINE` (<−2.57) / `UNIT_ROOT` (otherwise).
- **MFDFA**: `STRONG_MULTIFRACTAL` (Δh>0.30) /
  `MODERATE_MULTIFRACTAL` (>0.15) / `WEAK_MULTIFRACTAL` (>0.05) /
  `MONOFRACTAL` (otherwise).
- **HILLKS**: `GOOD_FIT` (D<0.5·crit) / `ACCEPTABLE_FIT` (<0.9·crit)
  / `POOR_FIT` (<1.3·crit) / `REJECT` (otherwise).
- **TSI**: `STRONG_BULL` (TSI>25) / `BULL` (>0) / `NEUTRAL` (|TSI|<5)
  / `BEAR` (>−25) / `STRONG_BEAR` (otherwise).

## Consequences

### Positive

- **First formal jump-detection test.** BIPOWER (Round 30) showed RV
  and BV side-by-side; BNSJUMP gives the Z-statistic and p-value that
  turn the ratio into a proper hypothesis test. Users can filter for
  jump-active tape periods without having to manually compare RV to BV.
- **Third stationarity axis.** ADF + KPSS already let the user
  cross-check stationarity under two different nulls. PPROOT adds a
  nonparametric third test robust to conditional heteroscedasticity —
  three-way agreement is a much stronger signal than any pair.
- **First multifractal-spectrum surface.** Hurst and DFA each reduce
  scaling to a single number. Real returns are multifractal — MFDFA's
  Δh reveals whether volatility clustering is spectrum-wide
  (monofractal) or concentrated in the tails (strongly multifractal).
  Δh > 0.3 is a strong diagnostic for heterogeneous scaling.
- **First tail goodness-of-fit surface.** HILLTAIL says "here's α̂";
  HILLKS says "here's how well Pareto actually fits your tail". A high
  D with a well-defined α̂ is a red flag that the Hill assumption is
  misspecified — the α̂ is quantitative nonsense even though it looks
  clean.
- **First double-smoothed momentum oscillator.** RSI/MACD/CCI all use
  at-most-double EMAs; TSI uses a 25/13 EMA sandwich on both signed
  and absolute ΔP. Cleaner zero-line crossovers than RSI, more stable
  divergence signals than MACD.

### Negative / Risks

- **Schema migration.** `create_research_tables_v39` is additive over
  v38, so peers on v38 who receive v39 rows via LAN sync will create
  the 5 new tables via the existing create-before-insert path. No
  back-compat break.
- **BNSJUMP at daily frequency is conservative.** The BNS theory was
  developed for intraday high-frequency data where the jump component
  is clearly separable. At daily frequency, large diffusion moves are
  hard to distinguish from true jumps — expect mostly `NO_JUMP` labels
  on liquid assets with occasional `WEAK_JUMP` on earnings/news days.
  This is honest behaviour, not a bug.
- **PPROOT lag truncation is automatic.** We use the Schwert (1989)
  rule q = floor(4·(n/100)^0.25) which gives q=5 for n≈253. Manual
  override is possible via a future parameter surface but not exposed
  now — keeps the public API narrow.
- **MFDFA is computationally heavier than DFA.** Three h(q)
  regressions instead of one, each requiring detrending at every scale.
  Well within the ~ms envelope per symbol since n=253 with 7 scales
  gives well under 10k fluctuation computations total.
- **HILLKS uses the two-sided tail** (absolute log-returns) because
  financial returns are typically near-symmetric. A one-sided variant
  would double the sample diagnostics. If a symbol has strong tail
  asymmetry, HILLKS' fit quality will be conservative — a real concern
  for highly skewed strategies, less so for equity indices.
- **TSI formula uses close-to-close changes** (ΔP = P_t − P_{t-1})
  rather than percentage changes. Blau (1991) defined it this way, and
  the ratio form is scale-invariant so the absolute unit doesn't
  matter. Percentage-change variants exist but would be a different
  oscillator.
- **Packet weight.** Each surface adds ~200–500 bytes per symbol.
  HILLKS is the lightest (6 scalars), MFDFA the heaviest (6 scalars +
  spectrum fit). Updated envelope numbers appear in the
  RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention established in
  Rounds 24–37 (UP=green for "favorable" label, DOWN=red for
  "adverse", AXIS_TEXT=neutral).
- **Palette aliases** avoid prior bindings. Verified no collisions on
  `BNSJUMP`, `PPROOT`, `MFDFA`, `HILLKS`, `TSI`, or their aliases.
- **All five surfaces use the same broker handler shape** that has been
  stable since Round 22.

## Verification

- `cargo test -p typhoon-engine --lib core::research::` — 524
  passing (up from 514 in Round 37, +10 new: 5 roundtrip + 5
  compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; no palette-alias
  collisions.
- BNSJUMP/PPROOT/MFDFA/HILLKS/TSI compute_oscillating use the ±0.5%
  oscillating fixture. Each asserts the returned label belongs to its
  regime set, scalars are finite when label is not INSUFFICIENT_DATA,
  and axis-specific invariants: BNSJUMP p∈[0,1]; PPROOT q≥1;
  MFDFA scales≥3; HILLKS KS stat≥0 and critical>0; TSI EMA params
  locked at 25/13.

## Packet envelope

After Round 38, single-symbol packet target envelope is **~64-129 KB**
(up from 63-127 in Round 37). Basket (10 symbols via BASKET) is
**~640-1290 KB** (up from 630-1270). Sub-block count grows 178 → 183.

Total HP-local research snapshot count after Round 38: **142**
(137 + 5). Total cross-symbol rank snapshots unchanged.
