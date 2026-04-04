# ADR-017: Drawing Tools

**Status:** Implemented | **Date:** 2026-03-24 | **Updated:** 2026-04-05

## Context
Chart annotation needed for technical analysis.

## Decision
73 drawing tool types across 11 toolbar groups. All accessible via command palette (~), toolbar menus, and right-click context menu.

### UX Features (2026-04-05)
- **Live preview**: ghost line/shape renders during placement for all drawing types
- **OHLC snap**: drawing endpoints magnetize to nearest candlestick O/H/L/C within 1.5% threshold
- **Undo/Redo**: Ctrl+Z / Ctrl+Shift+Z with drawing undo stack; drawing_styles kept in sync
- **Trashcan button**: red trash icon in toolbar for quick one-click delete
- **Color picker**: 8-color palette works for all drawing types via right-click menu
- **Status text**: "click point 2 of 3" format for multi-click tools with Esc cancel
- **Line width**: 1.0 / 1.5 / 2.0 / 3.0 px selector in toolbar, applied in render loop via effective_width
- **Line style**: Solid / Dashed / Dotted selector in toolbar, applied via draw_line() helper
- **Selection**: Click near drawing (8px threshold) to select; selected drawing highlights cyan + boosted width (~50 types)
- **Move/Drag**: Drag a selected drawing to reposition — all 73 types supported, blocks chart pan during drag
- **Delete selected**: Delete/Backspace removes selected drawing (or last drawing if none selected)

### Drawing Tool Categories (73 types)

| Category | Tools |
|----------|-------|
| **Lines** (8) | HLine, TrendLine, ExtendedLine, Ray, HRay, CrossLine, ArrowLine, TrendAngle |
| **Channels** (4) | Channel, ParallelChannel, RegressionChannel, FibChannel |
| **Fibonacci** (6) | FiboRetrace, FiboExtension, FibChannel, FibTimeZones, FibCircle, FibSpiral |
| **Shapes** (5) | Rectangle, Ellipse, Triangle, Highlighter, RotatedRectangle |
| **Gann** (2) | GannFan, GannBox |
| **Elliott** (2) | ElliottWave (5-point, 1-5 labels), AbcCorrection (3-point, A-B-C labels) |
| **Measure** (5) | InfoLine, PriceRange, DateRange, DatePriceRange, Ruler |
| **Patterns** (5) | Pitchfork, SchiffPitchfork, ModSchiffPitchfork, HeadShoulders, XabcdPattern |
| **Annotate** (11) | VLine, TextLabel, ArrowMarker, PriceLabel, Callout, CrossMarker, AnchorNote, Brush, Emoji, Flag, Balloon |
| **Position** (3) | LongPosition, ShortPosition, RiskRewardBox |
| **Cycles** (5) | CyclicLines, SineWave, TimeCycle, SpeedResistanceFan, SpeedResistanceArc |
| **Projection** (4) | Forecast, GhostFeed, AnchoredVwapLine, Signpost |
| **Curves** (3) | Polyline, ArcDraw, CurveDraw, PathDraw |
| **Other** (1) | SessionBreak, MagnetLevel |

## Consequences
- Pro: 73 drawing types covering all primary TA needs (near TradingView parity)
- Pro: Live preview during placement — user sees exactly what will be created
- Pro: OHLC snap ensures precise alignment with candlestick levels
- Pro: Full undo/redo with Ctrl+Z/Ctrl+Shift+Z
- Pro: Color picker works for all drawing types
- Pro: Session persistence for all drawings
- Pro: Elliott wave and pattern tools enable advanced technical markup
- Pro: Measurement tools (info line, date/price range) support trade planning
