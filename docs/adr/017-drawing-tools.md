# ADR-017: Drawing Tools

**Status:** Implemented | **Date:** 2026-03-24

## Context
Chart annotation needed for technical analysis.

## Decision
7 drawing tools: HLine, TrendLine, Fibonacci Retracement, VLine, Rectangle, Ray, Channel. All accessible via right-click context menu with click-to-place workflow. Color picker (8 colors) for last drawing. Delete key removes last, Clear All in menu. HLine drawings persist in session.json.

## Consequences
- Pro: Covers primary TA drawing needs
- Pro: Color customization
- Pro: Session persistence for horizontal lines
