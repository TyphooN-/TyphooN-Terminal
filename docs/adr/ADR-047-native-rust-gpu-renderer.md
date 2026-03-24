# ADR-047: Native Rust GPU Renderer — Eliminate WebKit

## Status: Accepted (2026-03-24)

## Context

The Tauri + WebKit architecture imposes a fundamental performance ceiling:

```
Rust backend → JSON serialize → Tauri IPC → JS JSON.parse → JS objects → WASM GPU
```

Every data path traverses 3 serialization layers and a single-threaded JS event loop. Despite extensive optimization (ADR-045, ADR-046), the WebKit bottleneck causes:
- **15-20s startup** with MTF grid (8 cells × 10K bars)
- **UI freezes** during grid population (main thread blocked by JSON.parse + object allocation)
- **~115ms per 10K bars** from cache to GPU (JSON serialize + IPC + parse + transform + pack)
- **~2GB memory** for 8-cell grid (WebKit + JS heap + WASM linear memory)

## Decision

**Replace WebKit entirely with a native Rust GPU renderer** using `egui` + `wgpu`.

### Architecture

- **Window management:** `winit` (cross-platform, Wayland/X11/macOS/Windows)
- **GPU rendering:** `wgpu` (Vulkan/Metal/DX12/OpenGL abstraction)
- **UI framework:** `egui` (immediate mode, renders via wgpu)
- **Chart engine:** Custom wgpu render pipeline (ported from `gpu-charts/src/lib.rs` WebGL2 shaders → WGSL)
- **Indicator computation:** Pure Rust on `&[f64]` slices (no Web Worker, no WASM bridge)
- **Data path:** SQLite → zstd decompress → zero-copy `&[f64]` → wgpu vertex buffer → VRAM

### Data Flow

```
SQLite (TTBR binary) → zstd::decode_all → &[u8]
  → reinterpret as &[f64] OHLCV (zero-copy)
    → wgpu::Buffer::write (DMA to VRAM)
      → GPU vertex/fragment shaders render candlesticks + indicators
```

No JSON, no IPC, no JS objects, no garbage collection.

### Performance Targets

| Metric | WebKit (current) | Native (target) |
|--------|-----------------|-----------------|
| Startup to interactive | 15-20s | < 2s |
| 10K bar chart render | ~200ms | < 5ms |
| MTF grid (8 × 10K bars) | 30-60s, freezes | < 2s, 60fps |
| Indicator computation | Worker + IPC (~50ms) | Direct Rust (< 10ms) |
| Memory (8-cell grid) | ~2GB | ~200MB |
| Real-time tick | ~50ms IPC round-trip | < 1ms |

## Migration Strategy

Parallel development alongside existing Tauri app:
1. Extract `src-tauri/src/` backend into `lib.rs` library crate
2. New `src/main.rs` creates native window, imports library
3. Both apps share identical `core/`, `broker/`, `strategies/` modules
4. Tauri app continues working until native reaches feature parity

## Implementation Phases

1. **Foundation** — eframe window, dark theme, single chart viewport with GPU candles
2. **Chart Engine** — All chart types (Candles/HA/Line/Bars/Renko), zoom/pan, axis labels
3. **Indicators** — Pure Rust SMA/EMA/KAMA/ATR/RSI/Fisher/BetterVolume
4. **UI Panels** — Command palette, positions, orders, watchlist, risk calculator
5. **Trading** — Order placement, SL/TP drag, martingale controls
6. **DARWIN Analytics** — Account viewer, portfolio dashboard, equity curves
7. **Advanced** — Drawing tools, backtest UI, screener, news
8. **Scripting** — MQL5/PineScript indicator layer (future)

## Consequences

- No more WebKit dependency (~200MB less in binary)
- No more JS/TypeScript — entire codebase is Rust
- Custom chart rendering requires porting 2K+ lines of WebGL2 shader code to WGSL
- egui's immediate mode paradigm differs from retained-mode HTML — UI code structure changes
- Existing WASM indicator worker (`indicator-worker.js`) replaced by native Rust computation
- 40K+ lines of JS frontend deprecated (kept for reference during migration)
