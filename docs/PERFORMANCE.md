# TyphooN Terminal — Performance

## Native GPU Renderer

The terminal uses egui + wgpu for direct GPU rendering. No WebView, no JavaScript, no IPC overhead.

### Benchmarks

| Metric | Value |
|--------|-------|
| Startup to interactive | < 2s (including SQLite cache load) |
| 10K bar chart render | < 5ms |
| 21 indicators on 10K bars | < 10ms total |
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
| Indicator computation (all 21) | < 10ms |
| egui Painter → wgpu surface | < 2ms |
| **Total: cache → pixels** | **< 15ms** |

### Why It's Fast

1. **Zero serialization** — data stays as Rust types from SQLite to GPU
2. **Immediate mode UI** — egui redraws only what changed, no DOM diffing
3. **Pre-computed indicators** — computed once on load, cached in ChartState
4. **Vulkan backend** — wgpu selects Vulkan on Linux (NVIDIA), Metal on macOS
5. **No garbage collection** — Rust ownership model, no GC pauses

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
