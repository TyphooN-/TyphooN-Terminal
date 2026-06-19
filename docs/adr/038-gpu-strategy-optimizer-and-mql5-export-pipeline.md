# ADR-038: GPU Strategy Optimizer & MQL5 Export Pipeline

> **⚠️ Partially superseded (2026-06).** The MQL5 *export* pipeline was removed (the `typhoon-transpiler` transpiler and GPU strategy optimizer remain). See [ADR-111](111-broker-scope-reduction-kraken-alpaca-only.md).

**Status:** Implemented
**Date:** 2026-03-26

## Context

MT5's built-in Strategy Tester is CPU-bound and single-threaded per optimization pass. Testing 10,000 parameter combinations across 5,000 bars with tick generation takes hours. There is no built-in way to distinguish "lucky" parameter spikes from genuinely robust edge — this is what Trade Like A Machine (OMS) sells as a separate product.

TyphooN-Terminal already has:
- All bar data in SQLite cache (ZSTD-compressed OHLCV)
- GPU compute infrastructure (wgpu compute shaders, `gpu_compute.rs`)
- 32+ indicator implementations in Rust
- MQL5 parser/frontend inside the `typhoon-transpiler` crate
- DARWIN analytics proving GPU batch computation works at scale (50K series)

## Decision

Build a GPU-accelerated strategy optimizer that tests millions of parameter combinations in seconds, with OMS-style robustness analysis, and exports optimized strategies to MQL5 for final validation.

### Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. STRATEGY DEFINITION                                          │
│                                                                 │
│ User defines:                                                   │
│ - Indicators (KAMA, Fisher, ATR, RSI, etc.)                    │
│ - Entry conditions (crosses, thresholds, combinations)          │
│ - Exit conditions (SL, TP, trailing stop, indicator-based)      │
│ - Filter conditions (ADX, volume, time-of-day)                  │
│ - Parameter ranges to optimize (e.g., KAMA period 5-50)        │
│                                                                 │
│ Strategy DSL or visual builder in egui                          │
└─────────────────────────┬───────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. GPU INDICATOR PRE-COMPUTATION                                │
│                                                                 │
│ For each parameter combination in the grid:                     │
│ - Upload bar data to VRAM (once, shared across all combos)     │
│ - Compute all indicator variants in parallel                    │
│   e.g., KAMA(5), KAMA(6), ..., KAMA(50) = 46 GPU dispatches   │
│ - Store indicator arrays in VRAM (no CPU round-trip)           │
│                                                                 │
│ 50 KAMA periods × 5000 bars = 250K values                      │
│ GPU computes all 250K in ~1ms                                   │
└─────────────────────────┬───────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. GPU STRATEGY EVALUATION (PARALLEL)                           │
│                                                                 │
│ One GPU thread per parameter combination:                       │
│ - Thread reads its parameter combo from params buffer           │
│ - Walks bars sequentially (but thousands of combos in parallel) │
│ - Evaluates entry/exit conditions using pre-computed indicators │
│ - Tracks equity curve, drawdown, trade count, win rate          │
│ - Writes results to output buffer                               │
│                                                                 │
│ 10,000 combos × 5,000 bars:                                    │
│ - MT5: 10,000 sequential passes = hours                         │
│ - GPU: 10,000 parallel threads = seconds                        │
│                                                                 │
│ Output per combo: [net_pnl, max_dd, sharpe, sortino, win_rate, │
│                    profit_factor, trade_count, avg_hold_time]    │
└─────────────────────────┬───────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. ROBUSTNESS ANALYSIS (OMS-equivalent)                         │
│                                                                 │
│ Filter lucky parameter spikes:                                  │
│ a) Neighbor stability: for each combo, check if neighbors       │
│    (±1 on each parameter) also produce similar results.         │
│    Isolated spikes = lucky. Plateaus = genuine edge.            │
│                                                                 │
│ b) Walk-forward validation: split data into in-sample (70%)     │
│    and out-of-sample (30%). Optimize on IS, validate on OOS.    │
│    Repeat with rolling windows. Only combos that work on        │
│    BOTH IS and OOS are genuinely robust.                        │
│                                                                 │
│ c) Monte Carlo permutation: shuffle trade order 1000×.          │
│    If equity curve shape is similar under shuffling,            │
│    the edge is not sequence-dependent.                          │
│                                                                 │
│ d) Parameter sensitivity score: variance of metric across       │
│    local neighborhood. Low variance = robust. High = fragile.   │
│                                                                 │
│ Output: ranked list of parameter combos with robustness score   │
│ Visualization: 3D parameter surface (like OMS heightmaps)       │
└─────────────────────────┬───────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ 5. MQL5 EXPORT                                                  │
│                                                                 │
│ Generate MQL5 source code for:                                  │
│                                                                 │
│ a) Indicators (if custom, not already in NNFX repo):            │
│    - .mqh (cross-platform logic)                                │
│    - .mq5 (MT5 wrapper)                                         │
│    - .mq4 (MT4 wrapper with #property strict)                   │
│                                                                 │
│ b) EA with optimized parameters:                                │
│    - Baked-in optimal parameter values as input defaults         │
│    - Entry/exit logic matching the strategy definition           │
│    - Risk management from TyphooN Risk Management System        │
│    - .set file with parameter values for MT5 tester              │
│                                                                 │
│ c) Validation report:                                           │
│    - Expected Sharpe, drawdown, win rate from GPU backtest       │
│    - Robustness score and sensitivity analysis                  │
│    - Walk-forward results                                       │
│    - "Run this one final MT5 tick-by-tick backtest to confirm"  │
└─────────────────────────┬───────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ 6. MT5 FINAL VALIDATION                                         │
│                                                                 │
│ User runs single MT5 Strategy Tester backtest with:             │
│ - Optimal parameters from GPU optimizer                         │
│ - Tick-by-tick mode (execution realism)                         │
│ - Confirms results match GPU within tolerance                   │
│ - If match: deploy to live. If mismatch: investigate slippage.  │
└─────────────────────────────────────────────────────────────────┘
```

### Performance Comparison

| Scenario | MT5 Tester | TyphooN GPU | Speedup |
|----------|-----------|-------------|---------|
| 100 param combos × 5K bars | ~5 min | ~0.5s | 600× |
| 10K param combos × 5K bars | ~8 hours | ~5s | 5,760× |
| 1M param combos × 5K bars | ~33 days | ~8 min | 5,760× |
| Walk-forward (10 windows × 10K combos) | ~80 hours | ~50s | 5,760× |
| Monte Carlo (1000 shuffles × best combo) | ~50 min | ~1s | 3,000× |

### Strategy DSL

```
strategy "NNFX Fisher Breakout" {
    timeframe: H4
    symbols: ["SLV", "CC", "GC", "CL"]

    indicators {
        kama = KAMA(period: 5..50, fast: 2, slow: 30)
        fisher = Fisher(period: 10..30)
        atr = ATR(period: 14)
        adx = ADX(period: 14)
    }

    entry LONG {
        fisher crosses_above 0
        kama.slope > 0
        adx.value > 20..35
    }

    entry SHORT {
        fisher crosses_below 0
        kama.slope < 0
        adx.value > 20..35
    }

    exit {
        trailing_stop: atr * 1.0..3.0
        take_profit: atr * 2.0..5.0
    }

    risk {
        max_risk_pct: 2.0
        max_positions: 1
    }
}
```

Parameter ranges (e.g., `5..50`, `1.0..3.0`) define the optimization grid.

### GPU Compute Shaders

**`strategy_eval.wgsl`** — One thread per parameter combination:
```wgsl
@group(0) @binding(0) var<storage, read> bars: array<Bar>;
@group(0) @binding(1) var<storage, read> indicators: array<f32>;  // pre-computed
@group(0) @binding(2) var<storage, read> params: array<ParamCombo>;
@group(0) @binding(3) var<storage, read_write> results: array<StrategyResult>;

@compute @workgroup_size(256)
fn eval_strategy(@builtin(global_invocation_id) id: vec3<u32>) {
    let combo_idx = id.x;
    if combo_idx >= arrayLength(&params) { return; }

    let p = params[combo_idx];
    var equity = 1000000.0;
    var peak = equity;
    var max_dd = 0.0;
    var wins = 0u;
    var losses = 0u;
    var in_trade = false;
    var trade_dir = 0;  // 1=long, -1=short
    var entry_price = 0.0;

    // Walk bars
    for (var i = p.lookback; i < bar_count; i++) {
        let kama_val = indicators[p.kama_offset + i];
        let fisher_val = indicators[p.fisher_offset + i];
        let fisher_prev = indicators[p.fisher_offset + i - 1];
        let atr_val = indicators[p.atr_offset + i];
        let adx_val = indicators[p.adx_offset + i];

        // Entry logic
        if !in_trade && adx_val > p.adx_threshold {
            if fisher_prev < 0.0 && fisher_val >= 0.0 && kama_slope > 0.0 {
                // Long entry
                in_trade = true; trade_dir = 1;
                entry_price = bars[i].close;
            }
            // ... short entry
        }

        // Exit logic (trailing stop)
        if in_trade {
            let sl_distance = atr_val * p.atr_sl_mult;
            // ... check stop hit, update trailing
        }

        // Track equity
        peak = max(peak, equity);
        max_dd = max(max_dd, (peak - equity) / peak);
    }

    results[combo_idx] = StrategyResult(equity, max_dd, wins, losses, ...);
}
```

### Robustness Scoring Shader

**`robustness.wgsl`** — Neighbor stability analysis:
```wgsl
// For each parameter combo, check if its N nearest neighbors
// produce similar results. Score = 1.0 - normalized_variance.
// Plateau (all neighbors similar) → high score
// Spike (neighbors much worse) → low score
```

### Implementation Phases

**Phase 1: Core Optimizer**
- Strategy definition struct (Rust, not DSL yet)
- GPU indicator pre-computation for parameter ranges
- GPU strategy evaluation shader
- Results readback and ranking

**Phase 2: Robustness Analysis** *(Implemented)*
- [x] Neighbor stability scoring — ROBUSTNESS_SHADER (GPU parallel)
- [x] Walk-forward validation — WALK_FORWARD_SHADER (GPU rolling windows)
- [x] Monte Carlo trade shuffling — MONTE_CARLO_SHADER (GPU parallel)
- 3D parameter surface visualization — deferred (egui_plot is 2D only)

**Phase 3: Strategy DSL** *(Implemented via MQL5 compiler)*
- [x] Parser for strategy definition language — MQL5 parser (pest grammar)
- [x] Compile DSL → GPU compute shader dispatch — WGSL codegen backend
- Visual strategy builder in egui — deferred (text-based MQL5/PineScript input works)

**Phase 4: MQL5 Export** *(Implemented)*
- [x] Generate .mqh/.mq5/.mq4 indicator source — mql5_export module + full 10-language transpiler matrix
- Generate EA source with optimal parameters
- Export .set files for MT5 tester
- Validation report generation

**Phase 5: OMS Feature Parity**
- Parameter sensitivity heatmaps
- Cluster analysis of profitable regions
- Multi-objective optimization (Sharpe + stability + low DD)
- Strategy portfolio optimization (multiple strategies, correlation-aware)

## Consequences

### Positive
- 1000-5000× faster than MT5 Strategy Tester
- Built-in robustness analysis (what OMS charges for separately)
- Seamless MQL5 export for production deployment
- Same indicator code runs on GPU (optimization) and MT5 (production)
- GPU cost: zero (user's existing graphics card)

### Negative
- Strategy evaluation on GPU requires deterministic floating-point (f32 vs MT5's f64)
- Complex strategies with many branches are harder to express in WGSL
- MT5 tick-by-tick execution effects (slippage, requotes) can't be simulated on GPU
- Strategy DSL is another language to maintain

### Mitigations
- f32 vs f64: final MT5 validation catches any precision-related divergence
- Complex branching: use CPU fallback for strategies that exceed GPU shader complexity
- Execution effects: GPU optimizer finds the parameter region, MT5 validates execution realism
- DSL maintenance: DSL compiles to both WGSL and MQL5, so it's the single source of truth
