# ADR-038: GPU Strategy Optimizer & MQL5 Export Pipeline

> **Historical foundation; partially superseded.** ADR-111 removed the MT5/MQL5
> export target in 2026-06. ADR-135 now governs the broader strategy research,
> backtesting, generation, robustness, portfolio, and guided NNFX workflow
> program. The fixed CPU/GPU backtester and optimizer described here are a
> first-draft foundation, not a complete system in either reference product's
> class.

**Status:** Partially implemented / superseded by [ADR-135](135-strategyquant-feature-parity-program.md)
**Date:** 2026-03-26

## Context

MT5's built-in Strategy Tester is CPU-bound and single-threaded per optimization pass. Testing 10,000 parameter combinations across 5,000 bars with tick generation takes hours. There is no built-in way to distinguish "lucky" parameter spikes from genuinely robust edge — this is what Trade Like A Machine (OMS) sells as a separate product.

TyphooN-Terminal already has:
- All bar data in SQLite cache (ZSTD-compressed OHLCV)
- GPU compute infrastructure (wgpu compute shaders, `gpu_compute.rs`)
- 32+ indicator implementations in Rust
- MQL5 parser/frontend inside the `typhoon-transpiler` crate
- DARWIN analytics proving GPU batch computation works at scale (50K series)

## Current implementation reality (2026-07-27)

The active implementation is deliberately narrower than the original proposal:

- `typhoon-engine/src/core/backtest.rs` provides five fixed bar-close strategies,
  a minimal `Strategy` callback, trade/equity/report output, bar-by-bar replay,
  SMA-cross grid search, and a fixed 70/30 SMA walk-forward routine.
- `typhoon-native/src/app/strategy_windows.rs` runs those strategies against the
  active chart, displays reports/trades/equity, exposes CPU/GPU parameter search,
  a two-parameter heatmap, and a walk-forward summary.
- `typhoon-native/src/gpu_compute/backtester.rs` and the inlined WGSL shaders
  accelerate fixed SMA/NNFX parameter combinations. They do not compile an
  arbitrary strategy graph, model realistic order execution, generate strategy
  populations, persist experiments, or run the full robustness catalog.
- The presence of separate robustness, walk-forward, and Monte Carlo pipelines
  is not proof that the complete original workflow is wired into the product.
- The transpiler parses indicator languages and can emit WASM/WGSL, but it is not
  the canonical strategy DSL/IR or no-code strategy builder proposed below.

The remainder of this ADR records the original direction. Unchecked work and
performance examples are historical design intent, not shipped capability or
measured evidence. ADR-135 replaces the broad completion plan with staged,
testable acceptance gates.

## Original decision

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

### Performance evidence

The original revision contained projected MT5-versus-GPU speedup figures. They
were not backed by a checked-in reproducible benchmark and are therefore not
retained as product claims. Future performance acceptance requires a versioned
dataset, strategy definition, precision/execution mode, hardware description,
warm-up policy, result checksum, and CPU-reference comparison. Throughput never
substitutes for simulation parity.

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

**`BACKTEST_EVAL_SHADER`** (WGSL, inlined in `typhoon-native/src/gpu_compute/shaders.rs`) — one thread per parameter combination:
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

**`ROBUSTNESS_SHADER`** (same module) — neighbor stability analysis:
```wgsl
// For each parameter combo, check if its N nearest neighbors
// produce similar results. Score = 1.0 - normalized_variance.
// Plateau (all neighbors similar) → high score
// Spike (neighbors much worse) → low score
```

### Implementation status of the original phases

**Core optimizer — first-draft foundation**
- [x] Fixed CPU SMA grid search and fixed GPU SMA/NNFX parameter evaluation.
- [x] Result readback, ranking, and a Fast × Slow heatmap.
- [ ] Canonical strategy definition/IR shared by CPU, GPU, GUI, and persistence.
- [ ] General indicator precomputation and arbitrary strategy-graph evaluation.

**Robustness analysis — partial foundation**
- [x] Fixed SMA 70/30 walk-forward analysis exposed in the native optimizer.
- [x] GPU result shape and shader foundations for robustness/Monte Carlo work.
- [ ] Proven end-to-end neighbor stability, trade-order Monte Carlo, parameter/data
  perturbation, multi-OOS, walk-forward matrix, and automatic rejection workflow.
- [ ] Reproducible robustness reports with deterministic seeds and versioned metrics.

**Strategy language and builder — missing**
- [ ] Versioned strategy IR/AST with typed blocks and validation.
- [ ] Visual no-code strategy builder and template/random-placeholder workflow.
- [ ] CPU reference interpreter and optional GPU lowering from the same semantics.

The MQL/Pine/etc. transpiler is useful adjacent language tooling, but does not
satisfy these strategy-model requirements.

**MQL5 export — removed from active scope**
- Historical export work was removed with MT5 in ADR-111. It is not a parity gate
  for the active Kraken + Alpaca native product.

**Broad strategy-research parity**
- Superseded by ADR-135, including generation/search, execution realism, robustness,
  databanks, analysis, portfolios, workflow automation, and extension boundaries.

## Consequences

### Positive
- Establishes a native CPU/GPU experimentation foundation inside the terminal.
- Reuses locally cached bars and the existing indicator/GPU infrastructure.
- Keeps accelerated parameter sweeps available while the reference simulator and
  general strategy model are built correctly.

### Negative
- Current fixed shaders and bar-close engine can create false confidence if
  presented as execution-realistic or general-purpose.
- GPU `f32` and CPU `f64` paths can diverge without golden parity vectors.
- Complex strategies and realistic event/order semantics are harder to lower to WGSL.
- A strategy IR, simulator, experiment store, and visual builder add substantial
  long-lived schema and compatibility obligations.

### Mitigations
- Treat a deterministic CPU event-driven simulator as the semantic reference.
- Require golden trade ledgers and CPU/GPU tolerance checks before acceleration is
  accepted for a strategy class.
- Make execution assumptions explicit and reject unsupported modes rather than
  silently approximating them.
- Version strategy, dataset, metric, and run manifests so results are reproducible.
- Follow ADR-135's staged correctness-first acceptance gates.
