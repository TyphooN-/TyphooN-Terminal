# TyphooN Terminal — Performance

## Native GPU Renderer

The terminal uses egui + wgpu for direct GPU rendering. No WebView, no JavaScript, no IPC overhead.

### Benchmarks

| Metric | Value |
|--------|-------|
| Startup to interactive | < 2s (including SQLite cache load) |
| 10K bar chart render | < 5ms |
| 46+ indicators on 10K bars | < 15ms total |
| MTF grid (4 cells x 5K bars) | < 50ms |
| Chart zoom/pan | 60fps, zero frame drops |
| Memory (single chart + indicators) | ~50-80MB |
| Memory (MTF 4-cell grid) | ~100-150MB |
| Binary size (release) | ~25MB |

### Data Pipeline

| Step | Time |
|------|------|
| SQLite read + zstd decompress | < 1ms |
| Bar struct construction | < 0.5ms |
| Indicator computation (all 46+) | < 15ms |
| egui Painter → wgpu surface | < 2ms |
| **Total: cache → pixels** | **< 20ms** |

### Why It's Fast

1. **Zero serialization** — data stays as Rust types from SQLite to GPU
2. **Immediate mode UI** — egui redraws only what changed, no DOM diffing
3. **Pre-computed indicators** — computed once on load, cached in ChartState
4. **Vulkan backend** — wgpu selects Vulkan on Linux (NVIDIA), Metal on macOS
5. **No garbage collection** — Rust ownership model, no GC pauses

### GPU DarwinIA Scan

Large DarwinIA datasets (>128MB) are processed via chunked batching in the GPU compute pipeline. The `compute_all_batches()` method splits return series into chunks that fit within wgpu buffer size limits, processes each chunk on the GPU, and merges the results. This enables scanning 50K+ DARWINs without exceeding VRAM constraints.

### Storage Compact (zstd-22)

The Storage Manager (`STORAGE` command) can recompress all bar_cache entries at zstd level 22 for maximum compression. Decompression speed is identical regardless of compression level — only on-disk storage shrinks. Progress is reported per 200 entries.

### Auto MT5 Sync

Bar data from MT5 (via BarCacheWriter EA) is automatically synced every ~5 minutes (1200 frames at 250ms). This picks up new bars without manual refresh.

### Cache Format

Bar data stored in TTBR (TyphooN Terminal Binary Record) format:
- 6 x f64 per bar (timestamp_ms, open, high, low, close, volume) = 48 bytes/bar
- zstd level 3 compression = ~3-5x ratio
- 10,000 bars = ~100KB compressed on disk

### GPU Backend Selection

wgpu auto-selects the best available backend:
1. Vulkan (Linux/Windows, preferred)
2. Metal (macOS)
3. DX12 (Windows fallback)
4. OpenGL (legacy fallback)

The `Unrecognized present mode` warnings on Wayland are harmless — it's a Vulkan extension for adaptive VSync that wgpu doesn't yet handle. Filtered to error level in the log subscriber.
