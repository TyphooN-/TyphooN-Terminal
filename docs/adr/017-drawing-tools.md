# ADR-017: Drawing Tools

**Status:** Implemented | **Date:** 2026-03-24 | **Updated:** 2026-03-31

## Context
Chart annotation needed for technical analysis.

## Decision
56 drawing tool types across 11 toolbar groups. All accessible via right-click context menu with click-to-place workflow. Color picker (8 colors) for last drawing. Delete key removes last, Clear All in menu. HLine drawings persist in session.json.

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
- Pro: Covers primary TA drawing needs (56 types, near TradingView parity)
- Pro: Color customization
- Pro: Session persistence for horizontal lines
- Pro: Elliott wave and pattern tools enable advanced technical markup
- Pro: Measurement tools (info line, date/price range) support trade planning
