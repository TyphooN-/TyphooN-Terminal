# ADR-005: Indicator Visual Parity with MT5

**Status:** Completed
**Date:** 2026-03-15
**Context:** TyphooN-Terminal must look identical to the MT5 NNFX setup for manual trading decisions.

## Visual Requirements

| Element | MT5 Default | TyphooN-Terminal | Status |
|---|---|---|---|
| Background | Black (#000000) | #000000 | ✓ |
| Candles up | Filled green | Filled #00FF00 | ✓ |
| Candles down | Filled red | Filled #FF0000 | ✓ |
| Grid | Dotted gray | Dotted #333, style 3 | ✓ |
| KAMA | White, width 2, PRICE_OPEN | #FFFFFF, w2, open | ✓ |
| 200 SMA | Yellow, width 1 | #FFFF00, w1 | ✓ |
| ATR Projection | Yellow, solid, width 2 | #FFFF00, solid, w2 | ✓ |
| Prev Candle H1/H4 | White, solid, width 2 | #FFFFFF, solid, w2 | ✓ |
| Prev Candle D1/W1 | Magenta, solid, width 2 | #FF00FF, solid, w2 | ✓ |
| Fisher bullish | MediumSeaGreen (#3CB371) | #3CB371 | ✓ |
| Fisher bearish | OrangeRed (#FF4500) | #FF4500 | ✓ |
| Fisher signal | DarkGray (#A9A9A9) | #A9A9A9 | ✓ |
| MTF H1 MA | Tomato (#FF6347) | #FF6347 | ✓ |
| MTF H4+ MA | Magenta (#FF00FF) | #FF00FF | ✓ |
| S/D demand | Filled green rectangle | Baseline series green fill | ✓ |
| S/D supply | Filled red rectangle | Baseline series red fill | ✓ |
| BetterVolume | Colored histogram (G/R/C/M/Y) | Separate pane, colored | ✓ |
| Fisher pane | Separate window | Separate chart instance | ✓ |

## Lessons Learned

See [INDICATOR_PORTING.md](../INDICATOR_PORTING.md) for technical details on each porting challenge.

## Key Decisions

1. **Fisher color segments**: Split into contiguous same-color line series (transition bar shared)
2. **S/D zones**: Baseline series with `baseValue` for bounded fill
3. **Sub-panes**: Separate chart instances with synced time scales
4. **All indicator axis labels disabled**: Prevents price axis clutter
5. **Indicators clip to last candle**: No drawing into future empty space
