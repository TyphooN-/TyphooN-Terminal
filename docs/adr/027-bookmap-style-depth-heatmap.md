# ADR-027: Bookmap-Style Depth Heatmap

## Status: Accepted current scope (updated 2026-05-26) - Snapshot L2/DOM and guarded live-depth rendering implemented

## Context

[Bookmap](https://bookmap.com/) provides a real-time heatmap visualization of order book depth over time. This shows liquidity at each price level as a color-intensity map, making it easy to spot:
- Large resting orders (bright spots = liquidity walls)
- Spoofing (orders that appear and disappear)
- Absorption (price stalls at a level with high volume)
- Iceberg orders (hidden liquidity revealed by trade flow)
- Vacuum zones (empty areas with no resting orders)

## Current Implementation

The terminal has per-symbol Bookmap windows, `/bookmap SYMBOL` command routing,
an orderbook DOM, and an Alpaca crypto orderbook snapshot path
(`AlpacaBroker::get_orderbook`). The current Bookmap view supports richer per-order
rendering for L3 (bid/ask markers, selected-order persistence, selected marker
highlight/ring, header selected-id + clear action, scroll list with
order_id/price/qty/age, click-to-copy/select interactions), live depth profile
overlays (25 bins + "L3 depth" label with tint distinction), and renders the
latest live orderbook snapshot only when the snapshot symbol matches the target
Bookmap window. Kraken L3 foundation (ws_v2_level3 with CRC, KrakenL3State,
received_at_ms for age) feeds the same update paths. Bookmap `Stream Depth`, the
Orderbook DOM, toolbar L2, and Order Flow Stream L2 share the same session
`dom_depth` preference instead of hardcoded depth values.

Kraken depth streaming is guarded to Kraken spot-pair symbols only. The UI
checks the loaded Kraken spot universe before enabling live depth, so equity
symbols and unsupported broker symbols cannot accidentally start Kraken L2
streams.

Retained streaming L2/L3 history is not part of the current Bookmap window scope.
The implemented view is a live/latest-depth visualizer with interaction state,
not a historical heatmap texture. Retained history requires broker data
entitlements and a dedicated ring-buffer/texture pipeline that is separate from
the normal chart renderer, so it should be reopened as a new implementation ADR
only when the feed entitlement and texture budget are available.

## Original Decision

Build a native Bookmap-style depth heatmap in the TyphooN Terminal using wgpu compute shaders for real-time rendering.

### Data Sources
- **Alpaca WebSocket**: Real-time order book snapshots (bid/ask depth at each price level)
- **Trade stream**: Executed trades overlaid on the heatmap
- **Historical**: Record snapshots to replay order book history

### Architecture

```
WebSocket → OrderBookSnapshot { price_levels: Vec<(f64, f64)> }
  → Ring buffer (last N snapshots, e.g., 3600 = 1 hour at 1/sec)
    → wgpu compute shader: map (time, price, volume) → pixel intensity
      → Texture render to screen
```

### Rendering
- **X-axis**: Time (scrolling left as new data arrives)
- **Y-axis**: Price levels (centered on current mid-price)
- **Color intensity**: Volume at each price level (blue→green→yellow→red gradient)
- **Trade markers**: Circles/dots for executed trades (green=buy, red=sell, size=volume)
- **Current spread**: Highlighted band between best bid/ask

### GPU Pipeline
```wgsl
// Compute shader: build heatmap texture from order book ring buffer
@compute @workgroup_size(16, 16)
fn build_heatmap(@builtin(global_invocation_id) id: vec3<u32>) {
    let time_idx = id.x;  // column = time snapshot
    let price_idx = id.y; // row = price level
    let volume = order_book_buffer[time_idx * price_levels + price_idx];
    let intensity = log(1.0 + volume) / max_log_volume;
    textureStore(heatmap_tex, vec2<i32>(id.xy), vec4<f32>(colormap(intensity), 1.0));
}
```

### Data Structure
```rust
struct OrderBookRing {
    snapshots: VecDeque<OrderBookSnapshot>,
    max_snapshots: usize,  // 3600 for 1 hour at 1Hz
    price_min: f64,
    price_max: f64,
    price_step: f64,        // tick size
}

struct OrderBookSnapshot {
    timestamp: i64,
    bids: Vec<(f64, f64)>,  // (price, volume)
    asks: Vec<(f64, f64)>,
    trades: Vec<Trade>,      // trades since last snapshot
}
```

### Features
1. **Real-time streaming**: New column every second from WebSocket
2. **Zoom**: Scroll to zoom price axis, Ctrl+scroll for time axis
3. **Replay**: Scrub back through recorded history
4. **Volume Profile**: Integrated side histogram showing cumulative volume at each price
5. **Delta coloring**: Option to color by net delta (buys - sells) instead of total volume
6. **Alert zones**: Draw horizontal lines at key levels, alert when liquidity appears/disappears

### Performance Requirements
- 60fps render of 3600×500 heatmap texture (1 hour × 500 price levels)
- GPU compute shader for texture generation (not CPU)
- Ring buffer with zero-copy GPU upload via `wgpu::Buffer::write`

## Reopen Criteria For Retained Depth History

Retained depth history is not an unfinished implementation item in this ADR. Reopen it as a new
ADR when all of these prerequisites exist:
- A broker entitlement that supplies continuous Level 2 snapshots for the
  target symbols.
- A ring buffer of order book snapshots and trade prints.
- A dedicated wgpu texture/compute pipeline for the depth heatmap.

## Consequences

- Requires wgpu compute shader support (Vulkan compute, available on user's NVIDIA hardware)
- Order book data storage: ~50MB/hour at 1Hz with 500 price levels
- May need dedicated GPU texture for the heatmap (separate from chart candle rendering)
