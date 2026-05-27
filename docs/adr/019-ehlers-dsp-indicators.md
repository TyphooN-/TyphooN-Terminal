# ADR-019: Ehlers DSP Indicators

**Status:** Implemented | **Date:** 2026-03-24

## Context
John Ehlers' digital signal processing indicators extract cycle and trend information from price data. Compatible with NNFX as confirmation/exit indicators.

## Decision
8 Ehlers indicators implemented from published papers:

**Overlay:** Super Smoother(10), Decycler(20), Instantaneous Trendline, MAMA/FAMA(0.5/0.05).

**Sub-Pane:** Even Better Sinewave(40), Cyber Cycle, CG Oscillator(10), Roofing Filter(10/48).

All use Ehlers' recursive filter formulas with 2-pole highpass, super smoother, and Hilbert transform components.

## Consequences
- Pro: Signal processing quality far exceeds standard indicators
- Pro: MAMA/FAMA adapts to market speed automatically
- Pro: EBSW identifies cycle vs trend mode
- Con: Parameters less intuitive than simple MA periods
