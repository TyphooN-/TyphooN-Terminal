# ADR-055: GPU-Accelerated DARWIN Analytics

**Status:** Accepted
**Date:** 2026-03-26

## Context

The terminal manages 50,317 DARWINs from the Darwinex FTP feed, each with up to 2,500+ days of return data. Key analytics — correlation matrix, universe screening, rolling VaR, Monte Carlo simulation — require processing millions of data points. CPU computation is sequential and slow for these workloads.

The terminal already has a `gpu_compute.rs` module with wgpu compute shaders for SMA/EMA. We extend this to DARWIN-specific analytics that exploit massive GPU parallelism.

## Decision

Add GPU compute pipelines for DARWIN analytics using wgpu compute shaders (WGSL).

### What Goes on GPU

| Analysis | Data Size | Parallelism | GPU Advantage |
|----------|-----------|-------------|---------------|
| **Batch statistics** (Sharpe/Sortino/DD for 50K DARWINs) | 50K × 500 floats = 100MB | 50K independent series | 50K threads simultaneously |
| **Pairwise correlation** (N×N matrix) | 5K × 5K = 25M pairs | Each pair independent | 25M thread dispatches in tiles |
| **Rolling VaR windows** | 50K × 500 × 30-window | Per-DARWIN, per-window | 50K × ~470 = 23M work items |
| **Monte Carlo VaR** | 1000 simulations × 252 days | Each simulation independent | 1000 parallel random walks |
| **Return distribution** (histogram, percentiles) | 50K series | Per-DARWIN | Parallel histogram binning |

### What Stays on CPU

| Analysis | Reason |
|----------|--------|
| File I/O (NAS reads) | Disk-bound, not compute-bound |
| SQLite queries | Lock-bound |
| Per-trade analysis | Irregular data, branch-heavy |
| String parsing | Inherently sequential |
| Small datasets (<1000 points) | GPU dispatch overhead exceeds computation |

### Memory Layout in VRAM

```
┌──────────────────────────────────────────────────────┐
│ Returns Buffer (storage, read-only)                  │
│ [d0_r0, d0_r1, ..., d0_rN, d1_r0, ..., dM_rN]     │
│ Flat array: M DARWINs × N max days                   │
│ Padded with 0.0 for DARWINs with fewer days          │
│ Size: 50K × 512 × 4 bytes = 100MB                   │
├──────────────────────────────────────────────────────┤
│ Lengths Buffer (storage, read-only)                  │
│ [len_0, len_1, ..., len_M]                           │
│ Actual day count per DARWIN (for bounds checking)     │
│ Size: 50K × 4 bytes = 200KB                          │
├──────────────────────────────────────────────────────┤
│ Stats Output Buffer (storage, read-write)            │
│ [mean, var, sharpe, sortino, maxdd, best, worst,     │
│  skew, kurt, total_ret]  × M DARWINs                │
│ 10 floats per DARWIN                                 │
│ Size: 50K × 40 bytes = 2MB                           │
├──────────────────────────────────────────────────────┤
│ Correlation Output Buffer (storage, read-write)      │
│ Tiled: 1024 × 1024 correlation coefficients          │
│ Size per tile: 4MB                                    │
│ Dispatched in tiles to avoid 10GB full matrix         │
└──────────────────────────────────────────────────────┘
```

### Compute Shaders (WGSL)

**1. Batch Statistics Shader** (`darwin_stats.wgsl`)
- One thread per DARWIN
- Computes: mean, variance, Sharpe, Sortino, max drawdown, best/worst day, skewness, kurtosis, total return
- Workgroup size: 256
- Dispatch: ceil(50K / 256) = 196 workgroups

**2. Pairwise Correlation Shader** (`darwin_corr.wgsl`)
- One thread per (i, j) pair in a tile
- Computes Pearson correlation between DARWIN i and DARWIN j
- Uses shared memory for mean/variance pre-computation
- Tile size: 1024 × 1024 (1M pairs per dispatch)
- For 5K DARWINs: 25 tile dispatches

**3. Rolling VaR Shader** (`darwin_rolling_var.wgsl`)
- One thread per (DARWIN, window_start) pair
- 30-day sliding window: compute VaR 95% (sort + percentile) per window
- Workgroup size: 256

**4. Monte Carlo Shader** (`darwin_monte_carlo.wgsl`)
- One thread per simulation path
- Uses GPU-side PRNG (PCG hash)
- Samples from historical return distribution
- 1000 simulations × 252 days forward projection

### Pipeline Architecture

```
CPU: Parse FTP files → f32 arrays
  ↓
GPU: Upload to Returns Buffer (queue.write_buffer)
  ↓
GPU: Dispatch batch_stats shader → Stats Output Buffer
  ↓
GPU: Dispatch correlation shader (tiled) → Correlation Output Buffer
  ↓
CPU: Readback Stats + Correlation via staging buffer
  ↓
UI: Display in DARWIN Browser / Correlation Matrix windows
```

### Performance Estimates

| Operation | CPU (single-thread) | GPU (RTX 3080) | Speedup |
|-----------|-------------------|----------------|---------|
| Stats for 50K DARWINs | ~25 seconds | ~50ms | 500× |
| 5K×5K correlation matrix | ~30 minutes | ~2 seconds | 900× |
| Rolling VaR (50K × 470 windows) | ~10 minutes | ~500ms | 1200× |
| Monte Carlo (1000 × 252 days) | ~2 seconds | ~5ms | 400× |

### Readback Strategy

- Stats (2MB) → single staging buffer copy, map_async, ~1ms
- Correlation tiles (4MB per tile) → stream back one tile at a time
- Results cached in `BgDarwinData` — only recompute when new data arrives

## Consequences

### Positive
- 50K DARWIN universe screening in <100ms instead of 25+ seconds
- Real-time correlation matrix updates as DARWINs are added
- Monte Carlo simulations run interactively (slider for horizon, immediate results)
- No external dependencies (wgpu compute shaders, already in the stack)
- VRAM usage modest (~100MB for full universe)

### Negative
- wgpu compute requires Vulkan/Metal/DX12 — no fallback for ancient GPUs (CPU path remains for these)
- WGSL shader debugging is harder than Rust debugging
- Sorting on GPU (for VaR percentile) requires bitonic sort or radix sort shader
- f32 precision only (sufficient for financial analytics — we don't need f64 on GPU)

### Neutral
- Existing CPU analytics remain as fallback / validation
- GPU results should match CPU within f32 tolerance (~1e-6 relative error)
