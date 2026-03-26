# ADR-050: GPU Compute Architecture — wgpu Compute Shaders for All Numerical Work

**Status:** Accepted (Phase 1 Implemented) | **Date:** 2026-03-24 | **Updated:** 2026-03-26

## Context

With the WebKit/JS layer eliminated, we have direct access to wgpu from Rust. All indicator computation, backtesting, Monte Carlo simulation, VaR calculation, and pattern detection can benefit from GPU parallelism. Modern GPUs have thousands of compute cores for embarrassingly-parallel workloads.

## Current Implementation Status

### Implemented (GPU)
- **DARWIN Batch Statistics** — 50K DARWINs × 10 metrics (Sharpe, Sortino, MaxDD, etc.) via `gpu_compute.rs`
- **Pairwise Correlation** — Tiled 1024×1024 Pearson correlation matrix via compute shader
- **GPU initialization** from eframe's wgpu render state (device/queue sharing with egui)

### Implemented (CPU, planned GPU migration)
- **Chart Indicators** — SMA, EMA, KAMA, RSI, Fisher, Bollinger, MACD, ATR, Stochastic, ADX, PSAR, WMA, HMA, Ichimoku, BetterVolume, OBV, SuperTrend, RVOL, ATR Projection, Previous Candle Levels, Supply/Demand Zones, AutoFibonacci
- **Monte Carlo VaR** — 1000 simulations × 252 days (CPU, in bg thread)
- **Rolling VaR** — 30-day sliding window per DARWIN (CPU)
- **Backtest Engine** — Bar-by-bar with optimization grid (CPU)

### Not Yet Implemented
- GPU indicator compute shaders (WGSL)
- GPU backtest/optimizer
- GPU Monte Carlo
- GPU harmonic pattern detection

## Decision

**All parallelizable numerical computation goes to GPU with CPU fallback for compatibility.**

This applies to BOTH the native GUI app AND any future TUI/CLI mode. The engine layer (`typhoon-engine`) owns the GPU compute abstraction — consumers (native app, CLI) initialize it from their respective wgpu contexts.

### What Goes to GPU

| Workload | Current | Target | Speedup |
|----------|---------|--------|---------|
| **Indicators (32+)** | CPU Rust loops (<10ms/10K bars) | GPU parallel per-bar | 100-1000x |
| **DARWIN Stats (50K)** | **GPU ✓** | Done | 500x |
| **Correlation Matrix** | **GPU ✓** (tiled) | Done | 900x |
| **Backtest Optimizer** | CPU grid search | GPU parallel eval all params | 1000x |
| **Monte Carlo VaR** | CPU single-threaded | GPU 100K parallel paths | 10000x |
| **Rolling VaR** | CPU per-DARWIN | GPU all DARWINs simultaneously | 1200x |
| **Harmonic Patterns** | Not implemented | GPU O(n^5) → parallel matching | 100x |
| **Volume Profile** | CPU binning | GPU parallel histogram | 50x |
| **Supply/Demand Zones** | CPU scan | GPU parallel impulse detection | 100x |

### What Stays on CPU

| Workload | Reason |
|----------|--------|
| File I/O (NAS, SQLite) | Disk-bound, not compute-bound |
| String parsing (FTP files, XLSX) | Inherently sequential |
| Small datasets (<1000 points) | GPU dispatch overhead exceeds computation |
| Broker API calls | Network-bound |
| Per-trade irregular analysis | Branch-heavy, irregular data |

### Architecture

```
SQLite cache → zstd decompress → &[f64] (CPU)
  → wgpu::Buffer::write_buffer (DMA to VRAM)
    → Compute shader dispatch (GPU)
      → Results in GPU storage buffer (VRAM)
        → Chart renderer reads directly from VRAM (zero copy back to CPU)
        → CPU reads back only for UI text display (map_async, small)
```

### Data Layout in VRAM

```wgsl
// Bar data: packed f64 array (6 values per bar)
struct BarData {
    timestamp: f64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

// Indicator output: one f64 per bar per indicator
// All 32+ indicators stored contiguously in one large buffer
// indicator_buffer[indicator_id * bar_count + bar_index] = value
```

### GPU Fallback Strategy

```
1. App startup: probe for wgpu compute support
2. If GPU available: initialize GpuCompute with device/queue
3. If GPU unavailable: set gpu_compute = None
4. All compute calls: match on gpu_compute presence
   - Some(gpu) → GPU path
   - None → CPU fallback (existing Rust implementations)
5. Results are identical within f32 tolerance (~1e-6)
```

### Migration Phases

**Phase 1 (DONE):** DARWIN batch stats + correlation via compute shaders
**Phase 2 (Next):** Indicator compute — port SMA/EMA/KAMA as proof of concept, then all 32+
**Phase 3:** Backtest optimizer — parallel strategy eval across parameter grid
**Phase 4:** Monte Carlo + analytics — GPU PRNG, 100K parallel paths
**Phase 5:** Pattern detection — harmonic patterns, S/D zones

## Consequences

- **Pro**: 100-10000x speedup for numerical work
- **Pro**: Bar data stays in VRAM — zero CPU-GPU round trips for rendering
- **Pro**: Optimizer can test millions of parameter combinations in seconds
- **Pro**: CPU fallback ensures compatibility with headless/SSH/old GPU systems
- **Con**: WGSL shaders are harder to debug than Rust
- **Con**: f64 on GPU requires `shader-f64` (NVIDIA); f32 sufficient for most analytics
- **Con**: GPU readback has latency — batch reads, don't read per-frame
