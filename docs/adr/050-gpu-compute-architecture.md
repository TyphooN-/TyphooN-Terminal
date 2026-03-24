# ADR-050: GPU Compute Architecture — wgpu Compute Shaders for All Numerical Work

**Status:** Accepted | **Date:** 2026-03-24

## Context

With the WebKit/JS layer eliminated, we now have direct access to wgpu from Rust. All indicator computation, backtesting, Monte Carlo simulation, VaR calculation, and pattern detection currently runs on CPU. Modern GPUs have thousands of compute cores that can parallelize these embarrassingly-parallel workloads.

## Decision

Move all parallelizable numerical computation to wgpu compute shaders. Store bar data and indicator results in GPU buffers (VRAM). CPU only handles UI layout and broker I/O.

### What Goes to GPU

| Workload | Current (CPU) | GPU Approach | Speedup |
|----------|--------------|-------------|---------|
| **Indicators (32+)** | Sequential Rust loops | Parallel compute per-bar | 100-1000x |
| **Backtest** | Bar-by-bar loop | Parallel strategy eval across parameter space | 100x |
| **Optimizer** | Grid search, sequential | Parallel eval all parameter combos simultaneously | 1000x |
| **Monte Carlo VaR** | Single-threaded | 100K simulations in parallel | 10000x |
| **Harmonic Patterns** | O(n^5) swing combos | Parallel pattern matching | 100x |
| **Correlation Matrix** | Nested loops | Parallel pairwise computation | 50x |
| **Volume Profile** | Sequential binning | Parallel histogram reduction | 50x |
| **Supply/Demand Zones** | Sequential scan | Parallel impulse detection | 100x |

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

### Compute Shader Pipeline

1. **Upload**: Bar data → GPU storage buffer (once per symbol load)
2. **Dispatch**: One compute shader per indicator type
   - Workgroup size: 256 threads
   - Each thread computes one bar's indicator value
   - Shared memory for lookback windows (SMA, EMA, etc.)
3. **Chain**: Indicators that depend on others read from GPU buffer directly
4. **Render**: Chart vertex shader reads indicator buffers for line rendering

### Phase 1: Indicator Compute (Immediate)
- SMA, EMA as proof of concept
- Bar data upload to VRAM
- Result readback for UI display

### Phase 2: All Indicators
- Port all 32+ indicators to WGSL compute shaders
- Parallel prefix sum for cumulative indicators (OBV, ATR)
- Shared memory for sliding window indicators

### Phase 3: Backtest + Optimizer
- Strategy evaluation as compute shader
- Parameter grid as 2D dispatch
- Equity curve in GPU buffer

### Phase 4: Monte Carlo + Analytics
- Random number generation on GPU (PCG family)
- 100K path simulation in single dispatch
- VaR/CVaR reduction on GPU

## Consequences

- **Pro**: 100-10000x speedup for numerical work
- **Pro**: Bar data stays in VRAM — zero CPU-GPU round trips for rendering
- **Pro**: Optimizer can test millions of parameter combinations in seconds
- **Pro**: Monte Carlo VaR becomes real-time (not batch)
- **Con**: WGSL shaders are harder to debug than Rust
- **Con**: f64 support requires `shader-f64` wgpu feature (available on NVIDIA)
- **Con**: GPU readback has latency — batch reads, don't read per-frame
- **Con**: Shared memory management for lookback windows adds complexity
