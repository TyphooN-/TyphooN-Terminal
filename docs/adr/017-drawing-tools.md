# ADR-017: Drawing Tools

**Status:** Implemented | **Date:** 2026-03-24 | **Updated:** 2026-03-30

## Context
Chart annotation needed for technical analysis.

## Decision
41 drawing tool types across 9 toolbar groups. All accessible via right-click context menu with click-to-place workflow. Color picker (8 colors) for last drawing. Delete key removes last, Clear All in menu. HLine drawings persist in session.json.

### Drawing Tool Categories (41 types)

| Category | Tools |
|----------|-------|
| **Lines** (8) | HLine, TrendLine, ExtendedLine, Ray, HRay, CrossLine, ArrowLine, TrendAngle |
| **Channels** (4) | Channel, ParallelChannel, RegressionChannel, FibChannel |
| **Fibonacci** (4) | FiboRetrace, FiboExtension, FibChannel, FibTimeZones |
| **Shapes** (4) | Rectangle, Ellipse, Triangle, Highlighter |
| **Gann** (2) | GannFan, GannBox |
| **Elliott** (2) | ElliottWave (5-point, 1-5 labels), AbcCorrection (3-point, A-B-C labels) |
| **Measure** (4) | InfoLine, PriceRange, DateRange, DatePriceRange |
| **Patterns** (3) | Pitchfork, HeadShoulders, XabcdPattern |
| **Annotate** (8) | VLine, TextLabel, ArrowMarker, PriceLabel, Callout, CrossMarker, AnchorNote, Brush |
| **Position** (2) | LongPosition (risk/reward box), ShortPosition (risk/reward box) |
| **Other** (1) | Polyline (multi-segment) |

## Consequences
- Pro: Covers primary TA drawing needs (41 types, near TradingView parity)
- Pro: Color customization
- Pro: Session persistence for horizontal lines
- Pro: Elliott wave and pattern tools enable advanced technical markup
- Pro: Measurement tools (info line, date/price range) support trade planning
