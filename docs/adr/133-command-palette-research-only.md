# ADR-133: Command Palette Is Research-Only

Date: 2026-07-09

## Status

Accepted.

## Context

TyphooN has two very different interaction surfaces:

1. Graphical trading/charting controls: chart type, tabs, drawing tools, indicator toggles, SL/TP planning, templates, screenshots, replay, MTF layout, and similar direct-manipulation UI.
2. Research and data-retrieval surfaces: fundamentals, filings, news, regulatory events, market structure, AI research packets, outlier scans, macro data, broker/account state, and research-oriented analytics.

The command palette had drifted into duplicating charting controls. That created stale parallel paths, confusing affordances, and accidental command use for features that already have better graphical controls.

## Decision

The command palette is a research launcher, not a chart-control surface.

Commands may open or fetch research/data surfaces. Commands must not duplicate graphical charting, drawing, or indicator controls.

Removed from the registered command palette and hidden command handlers:

- Drawing commands: all `DRAW_*`, `CLEAR_DRAWINGS`, `OBJECTS`, ruler-style drawing shortcuts.
- Chart-type/tab/export controls: `CANDLE`, `HEIKINASHI`, `LINE`, `OHLC`, `RENKO`, `NEW_TAB`, `CLOSE_TAB`, `EXPORT_CSV`, `COPY_CHART`, `SCREENSHOT`, `SHARE`, `REPLAY`.
- Chart layout/view toggles: `MTF`, direct timeframe commands, chart-context drawing suggestions, `COMPARE`, `PIVOTS`, `PREV_LEVELS`, and similar chart overlays.
- Indicator/preset/template commands: `INDICATORS`, `NNFX`, `RESET_IND`, `CONFLUENCE`, `COMPILE`, chart indicator toggles, chart indicator template commands, standalone technical-indicator windows, and standalone candlestick-pattern windows. Exception: `SMA_INTELLIGENCE` remains a research floating-window command because its purpose is outfit correlation research, not toggling chart overlays.
- SL/TP duplicate commands: `SET_SL`, `SET_TP` remain removed; SL/TP belongs to the navbar/chart UI.

Research commands remain valid. Examples: `NEWS`, `SEC`, `INSIDER`, `REG_SHO`, `HALTS`, `FUNDAMENTALS`, `ANALYST`, `SHORT_INTEREST`, `OPTIONS`, `FRED`, `CALENDAR`, `EV`, `OUTLIERS`, `ASKAI`, `ASKCLAUDE`, `ASKANTIGRAVITY`, `ASKCODEX`, `ASKGROK`, `EXPORT_PACKET`.

## Consequences

- Charting stays graphical: toolbar, navbar, chart body interactions, right-panel controls, and context menus own chart manipulation.
- The palette becomes less noisy and more reliable for research work.
- Hidden generic handlers are not allowed to keep removed charting/indicator commands alive after registry removal.
- Help/reference windows should reflect the registry directly; no separate drawing-command section.
- Regression tests should assert removed UI-owned commands stay out of the palette.

## Implementation notes

The cleanup removes the drawing command module from command dispatch, deletes the drawing-command handler source, prunes chart/indicator command registry entries, removes chart-only context suggestions, and keeps the real graphical controls intact.
