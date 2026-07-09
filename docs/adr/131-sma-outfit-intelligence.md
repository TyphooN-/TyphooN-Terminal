# ADR-131: SMA Outfit Intelligence Window

**Status:** Implemented
**Date:** 2026-07-03
**Related:** ADR-125 (chart-ui crate owns indicator math), ADR-133 (research-only
command palette), SMA Outfits research (https://github.com/unfairmarket/SMA-outfits), `typhoon-chart-ui/src/sma_outfits.rs`,
`typhoon-native/src/app/floating_windows/sma_intelligence.rs`

## Context

"SMA Outfits" (raultrades / **Unfair Market**,
github.com/raultrades/SMA-outfits) frames predetermined sets of SMA periods —
an "outfit", each period 1..=999 — as the execution-trigger systems that
institutional blackbox algorithms run on liquid equities, and argues for
public visibility into price interaction with those levels. The Apache-2.0
**sma-intelligence-platform** (niya-shroff) builds on the idea with
multi-outfit signal generation and confidence metrics over canonical outfits
such as 10/50/200 and 30/60/90.

TyphooN already computes SMAs in `typhoon-chart-ui::indicators`; what was
missing is the outfit-level view: is price wearing a given outfit (stacked
bullish/bearish), how far is it from each trigger SMA, and when did it last
cross one.

## Decision

Implement the concept natively — original Rust, no code from either project:

- **`typhoon_chart_ui::sma_outfits`** (pure, unit-tested): outfit spec
  parsing/validation (2–6 legs, periods 1..=999 per the SMA-outfits spec,
  canonical `a/b/c` label form), and `analyze_sma_outfit` over the chart's
  bars reporting per-leg SMA value, signed price distance (%), a ±0.5%
  **trigger band** flag, the most recent close↔SMA cross (bars-since +
  direction, 200-bar lookback), plus an outfit **stack state**
  (Bullish/Bearish/Mixed via strict ordering price > SMA₁ > … > SMAₙ) and an
  **alignment percentage** (pairwise bullish relations; exact ties count as
  neutral, so 100 = fully dressed bullish, 0 = fully dressed bearish,
  50 = flat). Partial history never claims a stack: any missing leg forces
  Mixed and an `insufficient history` marker.
- **`SMA_INTELLIGENCE` remains a research floating-window command** under
  ADR-133's research-only palette rule: per-outfit tables for
  the focused chart's symbol/timeframe, an outfit editor (add `10/50/200`
  style specs, remove, reset to the two canonical defaults), and a concept
  attribution footer. The intended research target is to identify the SMA outfits
  most correlated with a pair's behavior per the Unfair Market SMA Outfits work;
  the current implementation is a partial foundation. Custom outfits persist in the session JSON and are
  re-validated through the spec parser on restore, so a hand-edited session
  cannot smuggle out-of-range periods.

Deliberately not in scope: the platform's mocked signal taxonomy ("Dark Pool
Accumulation" etc.), ML extensions, and any order execution. This window is
descriptive bar math — signals stay in the operator's hands, consistent with
ADR-114's rejection of automated escalation.

## Consequences

- The outfit lens works on every symbol/timeframe the chart can load, at
  chart-data latency, with zero network or ML dependencies.
- Compute is on-demand while the window is open (a handful of SMA passes over
  the visible chart's bars — the same O(n) cost class as existing overlay
  indicators).
- Concept attribution travels with the code (module docs + window footer +
  this ADR). No code was taken from either referenced project; the
  sma-intelligence-platform is Apache-2.0 and the implementation here is
  independent, so no license text is required in NOTICE.
