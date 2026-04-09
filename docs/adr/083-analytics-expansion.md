# ADR-083: Analytics & Screening Expansion

**Status:** In Progress | **Date:** 2026-04-08

## Context

Comprehensive audit identified feature gaps vs TradingView/Bloomberg in options pricing, market breadth, relative strength ranking, and order flow. GPU compute path confirmed: 31 indicators fully wired (including 8 Ehlers), 0 unwired shaders (audit was corrected — all are active).

## Implemented

### Options Pricing Engine (engine/src/core/options.rs)
- Black-Scholes European option pricing
- Full Greeks: Delta, Gamma, Theta (per day), Vega (per 1% vol), Rho (per 1% rate)
- Newton-Raphson implied volatility solver (100 iterations, 1e-8 convergence)
- Put-call parity verified in tests
- Wired into Option Chain window: 7-column grid (Strike, Call, Put, Delta, Gamma, Theta, Vega)
- 8 tests: BS call/put pricing, Greeks call/put, put-call parity, IV roundtrip, edge cases

### Relative Strength Ranking (engine/src/core/screener.rs)
- `compute_relative_strength()`: ranks symbols by price performance over configurable lookback
- Returns sorted `Vec<RelativeStrengthEntry>` with symbol, return_pct, rank (1=strongest)
- 2 tests: ranking correctness, insufficient data handling

## GPU Path Status (Verified Complete)
- **31 GPU-accelerated indicators**: SMA, EMA, RSI, KAMA, ATR, MACD, Fisher, Stochastic, ADX, Ichimoku, WMA, HMA, CCI, Williams %R, OBV, Momentum, Parabolic SAR, Fractals, ATR Projection, Better Volume, Supply/Demand Zones, Anchored VWAP, Bollinger Bands + 8 Ehlers (Super Smoother, Decycler, Instantaneous Trendline, MAMA/FAMA, Even Better Sinewave, Cyber Cycle, CG Oscillator, Roofing Filter)
- **All have CPU fallback** (except Anchored VWAP — GPU only)
- **GpuBacktester struct exists** but has zero implementation (future: parallel parameter grid evaluation)

### Portfolio Metrics (engine/src/core/darwin.rs)
- Treynor Ratio: `(annualized_return - risk_free_rate) / beta`
- Jensen's Alpha: `(R_d - R_f) - β * (R_b - R_f)` (CAPM excess return)
- Added to `BenchmarkComparison` struct alongside existing alpha, beta, information_ratio

### Symbol Correlation Matrix (engine/src/core/screener.rs)
- `compute_symbol_correlation_matrix()`: N×N Pearson correlation from close price series
- Configurable window (0 = all bars, N = last N bars)
- Single-pass mean/var/cov, clamped [-1, 1]
- `CorrelationMatrix` struct: symbols + matrix + window_bars

### Volume Profile: Initial Balance (native/src/app.rs)
- Detects session start (first bar of last trading day)
- Computes IB High, IB Low, IB Range from first hour of session
- Displayed alongside POC and VAH/VAL

## Remaining Feature Gaps (Roadmap)
- [ ] Market Breadth indicators (Advance/Decline, McClellan — needs exchange-level data)
- [ ] Put/Call Ratio visualization (needs options volume data feed)
- [ ] GPU Monte Carlo VaR (parallel sampling — infrastructure exists)
- [ ] GPU Backtester (parallel parameter grid — struct exists)

## Consequences

- Options Chain now shows theoretical Greeks per strike — enables options strategy analysis
- Relative Strength enables momentum-based symbol selection
- GPU path verified at 31/31 indicators active — no dead code
- 467 total tests passing

See also: ADR-077 (Screener Framework), ADR-055 (GPU DARWIN Analytics)
