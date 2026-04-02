# ADR-017: Drawing Tools

**Status:** Implemented | **Date:** 2026-03-24 | **Updated:** 2026-04-02

## Context
Chart annotation needed for technical analysis.

## Decision
71 drawing tool types across 11 toolbar groups. All accessible via command palette (~), toolbar menus, and right-click context menu.

### UX Features (2026-04-02)
- **Live preview**: ghost line/shape renders during placement for all drawing types
- **OHLC snap**: drawing endpoints magnetize to nearest candlestick O/H/L/C within 1.5% threshold
- **Undo/Redo**: Ctrl+Z / Ctrl+Shift+Z with drawing undo stack
- **Trashcan button**: red trash icon in toolbar for quick one-click delete
- **Color picker**: 8-color palette works for all 60+ drawing types via right-click menu
- **Status text**: "click point 2 of 3" format for multi-click tools with Esc cancel

### Drawing Tool Categories (56 types)

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
- Pro: 71 drawing types covering all primary TA needs (near TradingView parity)
- Pro: Live preview during placement — user sees exactly what will be created
- Pro: OHLC snap ensures precise alignment with candlestick levels
- Pro: Full undo/redo with Ctrl+Z/Ctrl+Shift+Z
- Pro: Color picker works for all drawing types
- Pro: Session persistence for all drawings
- Pro: Elliott wave and pattern tools enable advanced technical markup
- Pro: Measurement tools (info line, date/price range) support trade planning
