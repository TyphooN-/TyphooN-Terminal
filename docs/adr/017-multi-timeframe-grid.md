# ADR-017: Multi-Timeframe Grid

**Status:** Implemented | **Date:** 2026-03-24

## Context
NNFX system requires viewing multiple timeframes simultaneously.

## Decision
4-cell 2x2 grid layout. Each cell is an independent chart viewport with the same indicator flags. Default: H4, D1, H1, W1. Toggle via View menu, toolbar MTF button, or ~ → MTF command. All cells share zoom/pan interaction.

## Consequences
- Pro: MT5-style multi-timeframe view
- Pro: Same indicator set across all cells
- Pro: Command presets now support 2×2, 3×3, 4×3, and 4×4 layouts via `MTF_2X2`, `MTF_3X3`, `MTF_4X3`, and `MTF_4X4`.
- Trade-off: grid layout is preset-driven, not arbitrary drag-resizable per cell.
