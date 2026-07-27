# ADR-062: Analytics & Screening Expansion

**Status:** Implemented for the scoped analytics; strategy research remains a first-draft foundation (ADR-135) | **Date:** 2026-04-08

## Context

Comprehensive audit identified feature gaps vs TradingView/Bloomberg in options pricing, market breadth, relative strength ranking, and order flow. GPU compute path confirmed: 31 indicators fully wired (including 8 Ehlers), 0 unwired shaders (audit was corrected — all are active).

## Implemented

### Options Pricing Engine (typhoon-engine/src/core/options.rs)
- Black-Scholes European option pricing
- Full Greeks: Delta, Gamma, Theta (per day), Vega (per 1% vol), Rho (per 1% rate)
- Newton-Raphson implied volatility solver (100 iterations, 1e-8 convergence)
- Put-call parity verified in tests
- Wired into Option Chain window: 7-column grid (Strike, Call, Put, Delta, Gamma, Theta, Vega)
- 8 tests: BS call/put pricing, Greeks call/put, put-call parity, IV roundtrip, edge cases

### Relative Strength Ranking (typhoon-engine/src/core/screener.rs)
- `compute_relative_strength()`: ranks symbols by price performance over configurable lookback
- Returns sorted `Vec<RelativeStrengthEntry>` with symbol, return_pct, rank (1=strongest)
- 2 tests: ranking correctness, insufficient data handling

## GPU Path Status (Verified Complete)
- **31 GPU-accelerated indicators**: SMA, EMA, RSI, KAMA, ATR, MACD, Fisher, Stochastic, ADX, Ichimoku, WMA, HMA, CCI, Williams %R, OBV, Momentum, Parabolic SAR, Fractals, ATR Projection, Better Volume, Supply/Demand Zones, Anchored VWAP, Bollinger Bands + 8 Ehlers (Super Smoother, Decycler, Instantaneous Trendline, MAMA/FAMA, Even Better Sinewave, Cyber Cycle, CG Oscillator, Roofing Filter)
- **All have CPU fallback** (except Anchored VWAP — GPU only)
- The GPU backtester now evaluates fixed SMA/NNFX parameter combinations; it is
  not a general strategy simulator/generator (ADR-038/135).

### Portfolio Metrics (typhoon-engine/src/core/darwin.rs)
- Treynor Ratio: `(annualized_return - risk_free_rate) / beta`
- Jensen's Alpha: `(R_d - R_f) - β * (R_b - R_f)` (CAPM excess return)
- Added to `BenchmarkComparison` struct alongside existing alpha, beta, information_ratio

### Symbol Correlation Matrix (typhoon-engine/src/core/screener.rs)
- `compute_symbol_correlation_matrix()`: N×N Pearson correlation from close price series
- Configurable window (0 = all bars, N = last N bars)
- Single-pass mean/var/cov, clamped [-1, 1]
- `CorrelationMatrix` struct: symbols + matrix + window_bars

### Volume Profile: Initial Balance (typhoon-native/src/app.rs)
- Detects session start (first bar of last trading day)
- Computes IB High, IB Low, IB Range from first hour of session
- Displayed alongside POC and VAH/VAL

### GPU Monte Carlo VaR (typhoon-native/src/gpu_compute.rs)
- `run_monte_carlo_gpu()`: dispatch method for existing MONTE_CARLO_SHADER
- PCG hash RNG on GPU, 256 threads/workgroup, N parallel simulations
- Returns sorted Vec of final equity values (VaR = percentile lookup)

### GPU Backtester (typhoon-native/src/gpu_compute.rs)
- `evaluate()` + `evaluate_nnfx()` implement the fixed SMA/NNFX sweep foundation.
- 5 WGSL pipelines are constructed: eval, nnfx, walk_forward, robustness, and
  Monte Carlo. Pipeline presence does not establish a complete wired robustness
  workflow; see ADR-038's corrected implementation status and ADR-135.
- BacktestResult: net_pnl, max_drawdown, sharpe, sortino, win_rate, profit_factor, trade_count, avg_hold_bars, robustness_score

## Data-Blocked Items
- Market Breadth indicators (Advance/Decline, McClellan) need an exchange-level breadth data feed.
- Put/Call Ratio visualization needs options volume data from CBOE or a comparable feed.

## Consequences

- Options Chain now shows theoretical Greeks per strike — enables options strategy analysis
- Relative Strength enables momentum-based symbol selection
- GPU path verified at 31/31 indicators active — no dead code
- Fixed GPU parameter evaluation and Monte Carlo VaR are available foundations;
  they do not close the broader StrategyQuant X / NNFX Algo Tester parity roadmap.

See also: ADR-056 (Screener Framework)
