# ADR-013: Auto-Load on Timeframe/Bar Count Change

**Status:** Implemented
**Date:** 2026-03-15

## Context

The UI had a "Load" button that users had to click after changing the timeframe or bar count dropdown. Every other trading terminal (MT5, TradingView, thinkorswim) auto-loads the chart when timeframe changes. The Load button was unnecessary friction.

## Decision

Remove the Load button. Auto-trigger chart load when:
- Timeframe dropdown (`#timeframe-select`) changes
- Bar count dropdown (`#bar-count`) changes
- Symbol entered via Enter key or autocomplete selection

## Changes

1. Extracted load logic into `triggerLoad()` — shared function with crypto ticker normalization
2. Added `change` event listeners on `#timeframe-select` and `#bar-count`
3. Updated autocomplete click and Enter key handlers to call `triggerLoad()` directly
4. Removed `#btn-load-chart` button from HTML, CSS, and JS

## Consequences

- **Pro**: Matches user expectation from MT5/TradingView — select timeframe, chart loads instantly
- **Pro**: Fewer UI elements — cleaner top bar
- **Pro**: Pre-cached timeframes (from background pre-fetch) appear near-instantly
- **Con**: Accidental timeframe changes trigger a load (acceptable — data is cached)
