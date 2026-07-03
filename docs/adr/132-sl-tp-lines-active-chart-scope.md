# ADR-132: SL/TP Trade Lines Are Active-Chart-Scoped

**Status:** Implemented
**Date:** 2026-07-03
**Related:** ADR-125 (chart-ui render layer), `typhoon-chart-ui/src/render.rs`
(`PriceViewGeometry`), `typhoon-native/src/app/trade_ops.rs`,
`typhoon-native/src/app/app_runtime_central_panel.rs`

## Context

SL/TP planning lines were app-global state (`sl_price` / `tp_price`) painted
into **every** chart, including every MTF grid cell, regardless of symbol.
Two live defects followed:

1. **Risk**: Buy Lines / Sell Lines drawn on one chart also appeared on every
   other open chart. In MTF view a user could read them as levels for a
   different symbol/timeframe — and the order paths (`quick_trade_plan`,
   `sync_current_position_exits`) consumed the global prices against the
   *active* chart's symbol with no ownership check, so lines drawn for one
   symbol could arm a trade on another.
2. **Broken dragging**: line hit-testing re-derived the price↔y mapping from
   legacy `price_pan`/`price_zoom` over the full panel rect. The real render
   mapping excludes sub-panes and the 24px time axis, extends the price range
   with live quotes and enabled overlay indicators, honors log scale, and is
   camera-authoritative in free-look — so the derived y disagreed with the
   painted pixels by dozens of pixels and grabs missed (the gesture fell
   through to chart pan). MTF cells had no line-drag path at all.

## Decision

1. **Ownership**: every path that sets SL/TP lines records
   `trade_lines_symbol` (the active chart's normalized symbol). Lines render
   and hit-test **only on the active chart, and only while its symbol matches
   the owner** — one cell in MTF, the single chart otherwise. The order paths
   hard-refuse on mismatch with an explicit error naming both symbols.
2. **Exact geometry**: `draw_chart` now returns `PriceViewGeometry` — the
   chart-rect + final price range + log flag it actually painted with — and
   the native callers stash it per chart (`ChartState::last_price_geometry`).
   A unified pre-pass (single + MTF) grabs a line when the press lands within
   8px of its painted y, applies drag deltas through the same geometry
   (`drag_price` is exact under linear and log scale), and both chart
   body-drag paths yield while a line drag is live — ending the split-gesture
   fights where the camera panned underneath the line.

## Consequences

- Lines can no longer be misread as levels on, or arm trades against, a
  chart they weren't drawn for. Switching the active chart to a different
  symbol hides the lines (prices stay in the trading-panel inputs) and
  re-focusing the owner chart shows them again.
- Dragging tracks the rendered line 1:1 in single and MTF views, on linear
  and log scales, with any sub-pane mix, in free-look or follow mode.
- Hit-testing uses last frame's geometry (immediate-mode: one frame stale);
  at interactive frame rates this is imperceptible and strictly better than
  the previous re-derivation, which was wrong by construction.
