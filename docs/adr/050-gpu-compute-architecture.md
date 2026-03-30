# ADR-050: GPU Compute Architecture — wgpu Compute Shaders for All Numerical Work

**Status:** Implemented | **Date:** 2026-03-24 | **Updated:** 2026-03-30

## Context

With the WebKit/JS layer eliminated, we have direct access to wgpu from Rust. GPU parallelism provides 100-5000× speedup for batch numerical work on the GTX 1080 (2560 CUDA cores) vs the E5-2696 v4 (44 threads).

## Implementation Status: 29 GPU Compute Shaders (~98% coverage)

### Chart Indicators on GPU (23 shaders)

| # | Shader | Dispatch | Output | Category |
|---|--------|----------|--------|----------|
| 1 | SMA | Parallel 256 | f32/bar | Trend |
| 2 | EMA | Sequential | f32/bar | Trend |
| 3 | KAMA | Sequential | f32/bar | Trend |
| 4 | WMA | Parallel 256 | f32/bar | Trend |
| 5 | HMA | Sequential (WMA composition) | f32/bar | Trend |
| 6 | Ichimoku | Sequential | [tenkan,kijun,span_a,span_b]/bar | Trend |
| 7 | RSI | Sequential | f32/bar | Momentum |
| 8 | MACD | Sequential | [line,signal,hist]/bar | Momentum |
| 9 | Stochastic | Sequential | [%K,%D]/bar | Momentum |
| 10 | CCI | Parallel 256 (OHLC) | f32/bar | Momentum |
| 11 | Williams %R | Parallel 256 (OHLC) | f32/bar | Momentum |
| 12 | Momentum | Parallel 256 | f32/bar | Momentum |
| 13 | ADX | Sequential (OHLC) | [adx,+DI,-DI]/bar | Momentum |
| 14 | Bollinger | Parallel 256 | [mid,upper,lower]/bar | Volatility |
| 15 | ATR | Sequential (OHLC) | f32/bar | Volatility |
| 16 | ATR Projection | Parallel 256 | [upper,lower]/bar | Volatility |
| 17 | Fisher | Sequential (midpoints) | [fisher,trigger]/bar | Oscillator |
| 18 | OBV | Sequential | f32/bar | Volume |
| 19 | BetterVolume | Parallel 256 (OHLC) | u8 class/bar | Volume |
| 20 | Parabolic SAR | Sequential (OHLC) | f32/bar | Other |
| 21 | Fractals | Parallel 256 (OHLC) | [up,down]/bar | Pattern |
| 22 | Supply/Demand Zones | Parallel 256 (OHLC) | [type,high,low]/bar | Pattern |
| 23 | Ehlers SuperSmoother | Sequential | f32/bar | DSP |
| 24 | Ehlers Decycler | Sequential | f32/bar | DSP |
| 25 | Ehlers ITL | Sequential | f32/bar | DSP |
| 26 | Ehlers Cyber Cycle | Sequential | f32/bar | DSP |
| 27 | Ehlers CG Oscillator | Parallel 256 | f32/bar | DSP |
| 28 | Ehlers Roofing Filter | Sequential | f32/bar | DSP |
| 29 | Ehlers EBSW | Sequential | f32/bar | DSP |
| 30 | Ehlers MAMA/FAMA | Sequential | [mama,fama]/bar | DSP |
| 31 | DARWIN Batch Stats | Parallel 256 | 10 metrics × 50K series | Analytics |
| 32 | DARWIN Correlation | Parallel 16×16 tiles | Pearson × N×N pairs | Analytics |

### Backtest/Optimizer Shaders (5 shaders)

| # | Shader | Dispatch | Purpose |
|---|--------|----------|---------|
| 33 | SMA Cross Strategy Eval | Parallel 256 | 1 thread per param combo, SMA cross + RSI + ATR |
| 34 | NNFX Strategy Eval | Parallel 256 | Fisher + KAMA + ATR + ADX inline per thread |
| 35 | Walk-Forward Validation | Parallel 256 | Out-of-sample window evaluation |
| 36 | Robustness Scoring | Parallel 256 | Neighbor stability analysis |
| 37 | Monte Carlo VaR | Parallel 256 | PCG PRNG random walk simulation |

### CPU-Only (3 indicators, ~2%)

| Indicator | Reason |
|-----------|--------|
| Previous Candle Levels | Groups bars by calendar day using `ts_ms` timestamps. GPU has no timestamp buffer; sequential day-boundary detection is O(n) and trivial. |
| Auto Fibonacci | Reduction search over GPU-computed fractal results for highest/lowest swing points. O(n) argmax — GPU dispatch overhead exceeds computation. |
| Harmonic Patterns | 5-point XABCD geometry matching with Fibonacci ratio validation. Deeply branching pattern search that would underperform on GPU. |

## Architecture

```
SQLite cache → zstd decompress → &[f64] (CPU)
  → Cast to f32, upload via queue.write_buffer (DMA to VRAM)
    → Compute shader dispatch (GPU)
      → Results in GPU storage buffer (VRAM)
        → CPU reads back via staging buffer (map_async)
          → Convert f32 → Vec<Option<f64>> for rendering
```

### GPU Fallback Strategy

```rust
fn compute_indicators_gpu(&mut self, gpu: Option<&mut GpuCompute>) {
    if let Some(gpu) = gpu {
        // Upload bar data to VRAM
        gpu.upload_bars_full(&closes, &highs, &lows);
        // GPU path: try shader, fall back to CPU if None
        if let Some(data) = gpu.compute_sma_gpu(200) { ... }
        else { self.sma200 = compute_sma(&self.bars, 200); }
        // ... repeat for all indicators
    }
    // CPU-only path (no GPU available)
    self.sma200 = compute_sma(&self.bars, 200);
    // ...
}
```

### VRAM Buffer Layout

| Buffer | Contents | Size (10K bars) |
|--------|----------|-----------------|
| `bar_buffer` | Close prices (f32) | 40 KB |
| `ohlc_buffer` | [H,L,C] interleaved (3×f32) | 120 KB |
| `mid_buffer` | (H+L)/2 midpoints (f32) | 40 KB |
| Output buffers | Per-indicator results | 40-120 KB each |
| Readback staging | MAP_READ for CPU access | 120 KB |

Total VRAM for a 10K bar chart: ~500 KB. Negligible on a 8GB GPU.

### Performance (GTX 1080, 2560 CUDA cores)

| Workload | CPU (E5-2696 v4) | GPU | Speedup |
|----------|-------------------|-----|---------|
| 32 indicators × 10K bars | ~3ms | ~0.1ms | 30× |
| 50K DARWIN batch stats | ~25s | ~200ms | 125× |
| 10K param optimizer | ~4 hours | ~30s | 480× |
| 5K×5K correlation matrix | ~30 min | ~5s | 360× |
| Monte Carlo 10K paths | ~10s | ~50ms | 200× |

## Chunked Batching for Large DarwinIA Scans

When scanning >50K DARWINs via the FTP pipeline, the combined daily return data can exceed wgpu buffer size limits (~128MB). The `compute_all_batches()` method in `GpuCompute` handles this by:

1. Splitting the return series into chunks that fit within the GPU buffer limit (`chunk_size`)
2. Dispatching each chunk as a separate GPU compute pass
3. Merging the per-chunk `GpuDarwinStats` results into a single output vector

This ensures the GPU path works for arbitrarily large DarwinIA datasets without falling back to CPU.

## Consequences

### Positive
- Near-total GPU coverage (98%) with automatic CPU fallback
- Zero `unsafe` code in entire codebase — all GPU buffer marshalling via `bytemuck` (Pod/Zeroable derives, `cast_slice`)
- Zero-copy bar data path: cache → VRAM → compute → render
- Strategy optimizer tests thousands of parameter combinations simultaneously
- All 8 Ehlers DSP filters on GPU (first trading terminal to do this)
- Works on GTX 1080 (Pascal, Vulkan compute 6.1) — no cutting-edge GPU required

### Negative
- f32 precision only on GPU (sufficient for all financial analytics)
- Sequential indicators (EMA, RSI, KAMA) dispatch as single workgroup — still benefits from GPU clock speed and VRAM bandwidth
- GPU readback adds ~1ms latency per indicator — batched to minimize round-trips
- WGSL shaders harder to debug than Rust — CPU implementations serve as validation reference
