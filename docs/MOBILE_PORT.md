# TyphooN-Terminal Mobile Port — Android/iOS Plan

## Goal

Replace TradingView mobile with TyphooN-Terminal on Android/iOS. Full charting, order management, risk analysis, and outlier scanners — same feature set as desktop. Leverage the existing SQLite cache for offline-capable bar data (sideload from desktop, no server re-download).

---

## Option Analysis

### Option 1: Tauri Mobile (Recommended)

**Tauri 2.0 already supports Android + iOS** as first-class targets. The existing Rust backend compiles to native ARM, and the WebView frontend renders in the platform's native WebView (WKWebView on iOS, Android WebView).

| Aspect | Details |
|---|---|
| **Effort** | Low-Medium — same codebase, add mobile targets |
| **Shared code** | 95%+ — Rust backend + JS frontend identical |
| **Binary size** | ~15-20MB (same as desktop, no Electron) |
| **Performance** | Native Rust + GPU WebGL (where supported) |
| **Distribution** | APK sideload (Android), TestFlight (iOS) |
| **Limitations** | No system tray, limited background tasks |

**Steps:**
```bash
# Add mobile targets to existing Tauri project
cargo tauri android init
cargo tauri ios init

# Build
cargo tauri android build
cargo tauri ios build
```

The main changes needed:
1. **Responsive CSS** — right panel collapses to bottom sheet on narrow screens
2. **Touch events** — chart pan/zoom via touch (lightweight-charts supports this natively)
3. **File paths** — use platform-specific app data directories instead of `~/.config/`
4. **No GPU Wasm** — WebGL2 may not be available on all mobile WebViews; fall back to lightweight-charts CPU rendering

### Option 2: React Native + Rust FFI

Rebuild the frontend in React Native while keeping the Rust backend via FFI bridge.

| Aspect | Details |
|---|---|
| **Effort** | High — full frontend rewrite |
| **Shared code** | ~50% — only Rust backend shared |
| **Binary size** | ~30-50MB (React Native + Hermes) |
| **Performance** | Good but not native WebView |
| **Distribution** | App Store / Play Store |

**Not recommended** — too much work for minimal benefit over Tauri Mobile.

### Option 3: PWA (Progressive Web App)

Wrap the existing frontend as a PWA with service worker for offline support.

| Aspect | Details |
|---|---|
| **Effort** | Low — add manifest + service worker |
| **Shared code** | 100% frontend, needs hosted backend |
| **Binary size** | 0 (web) |
| **Performance** | Depends on browser |
| **Distribution** | URL → "Add to Home Screen" |
| **Limitations** | No local Rust backend, needs API proxy server |

**Not recommended** — violates local-first principle (needs server for Rust backend).

---

## Recommended Architecture: Tauri Mobile

```
┌─────────────────────────────────────┐
│         TyphooN-Terminal Mobile     │
│  ┌─────────┐ ┌──────────────────┐  │
│  │ WebView │ │   Rust Backend   │  │
│  │ (HTML/  │ │ (same alpaca.rs, │  │
│  │  JS/CSS)│ │  cache.rs, etc.) │  │
│  └────┬────┘ └────────┬─────────┘  │
│       │    Tauri IPC   │            │
│       └────────┬───────┘            │
│                │                    │
│  ┌─────────────┴──────────────┐     │
│  │      SQLite Cache          │     │
│  │  (sideloaded from desktop  │     │
│  │   OR fetched incrementally)│     │
│  └────────────────────────────┘     │
└─────────────────────────────────────┘
```

---

## Bar Data Strategy: Sideload vs Fetch

### Sideload from Desktop (Primary — Zero API Cost)

The desktop SQLite cache (`typhoon_cache.db`) contains all bar data in compressed binary format (TTBR + zstd). This file can be copied to mobile for instant access:

```
Desktop: ~/.config/typhoon-terminal/cache/typhoon_cache.db
  → Copy via USB, Syncthing, rsync, or cloud sync
Mobile:  /data/data/org.typhoon.terminal/cache/typhoon_cache.db
```

**Benefits:**
- Zero API calls on mobile — all data pre-cached
- Full history depth (whatever desktop has cached)
- Works offline (airplane mode trading analysis)
- ~50-100MB for typical cache (zstd compressed)

**Sync strategy:**
1. **Manual**: USB transfer or file manager copy
2. **Auto (Syncthing)**: Syncthing watches the cache directory, syncs to phone over LAN
3. **Auto (rsync over SSH)**: `rsync -avz ~/.config/typhoon-terminal/cache/ phone:~/typhoon-cache/`
4. **Cloud**: Dropbox/Google Drive sync of the cache file

### Incremental Fetch on Mobile (Secondary — Light API Usage)

When the sideloaded cache is stale (e.g., last synced yesterday), mobile can do incremental fetches:
- Same `get_bars_incremental` logic as desktop
- Fetches only bars newer than the second-to-last cached bar
- Typically 1-2 API calls per symbol (vs 13+ for cold start)
- Respects the same adaptive rate limiter

### Compression Analysis

| Cache Content | Raw Size | zstd Compressed | Transfer Time (WiFi) |
|---|---|---|---|
| 10 symbols × 9 TFs, 1000 bars each | ~43 MB | ~4 MB | <1 second |
| 50 symbols × 9 TFs, 2000 bars each | ~430 MB | ~40 MB | ~5 seconds |
| 100 symbols × 9 TFs, 5000 bars each | ~2.1 GB | ~200 MB | ~30 seconds |

The existing zstd level 9 compression (30:1 ratio on binary bar data) makes sideloading practical even for large caches.

---

## Mobile-Specific Changes Needed

### 1. Responsive Layout

```css
/* Mobile breakpoint: stack right panel below chart */
@media (max-width: 768px) {
  #main-layout { flex-direction: column; }
  #right-panel { width: 100%; height: auto; max-height: 40vh; }
  #chart-stack { min-height: 50vh; }
  #panel-splitter { display: none; } /* no vertical splitter on mobile */
}
```

### 2. Touch Controls

- **Pinch zoom** on chart (lightweight-charts supports natively)
- **Swipe left/right** to switch tabs
- **Long press** for crosshair (instead of mouse hover)
- **Bottom sheet** for order entry (instead of floating window)
- **Pull to refresh** for dashboard update

### 3. Platform File Paths

```rust
#[cfg(target_os = "android")]
fn get_cache_dir() -> PathBuf {
    // Android app-specific storage
    PathBuf::from("/data/data/org.typhoon.terminal/cache")
}

#[cfg(target_os = "ios")]
fn get_cache_dir() -> PathBuf {
    // iOS Documents directory
    dirs::document_dir().unwrap_or_default().join("typhoon-terminal")
}
```

### 4. Reduced Feature Set (Phase 1 Mobile)

Focus on essential trading features for mobile:

| Feature | Mobile Phase 1 | Mobile Phase 2 |
|---|---|---|
| Chart (candlestick) | ✅ | ✅ |
| MTF Grid (2×2) | ✅ | ✅ (3×2) |
| Positions panel | ✅ | ✅ |
| Orders panel | ✅ | ✅ |
| Place order (market/limit) | ✅ | ✅ |
| Close position | ✅ | ✅ |
| Watchlist | ✅ | ✅ |
| Indicators (SMA/EMA/KAMA) | ✅ | ✅ |
| Risk dashboard | ❌ | ✅ |
| Command palette (Ctrl+K) | ❌ | ✅ (search bar) |
| Outlier scanners | ❌ | ✅ |
| Drawing tools | ❌ | ✅ (Phase 2) |
| GPU chart engine | ❌ | Maybe (WebGL2 support varies) |

---

## Timeline Estimate

| Phase | Work | Scope |
|---|---|---|
| **Phase 1** | Tauri mobile init + responsive CSS + touch events | 2×2 grid, basic trading |
| **Phase 2** | SQLite cache sideload + incremental sync | Offline-capable charting |
| **Phase 3** | Full command palette + risk tools | Feature parity |
| **Phase 4** | App Store / Play Store submission | Distribution |

---

## Alternative: TUI on Termux (Android — Already Works)

The CLI/TUI (`typhoon` binary) can run today on Android via Termux:

```bash
# Install Rust in Termux
pkg install rust

# Build CLI
cd TyphooN-Terminal/cli
cargo build --release

# Run
./target/release/typhoon
```

This gives immediate Android access to:
- ASCII candlestick charts
- Position/order management
- Watchlist with live quotes
- Risk dashboard
- Account info + MT5 import

No App Store needed. Works over SSH too (manage trades from phone via SSH to desktop).
