# ADR-105: Performance Optimization Plan and Focus Areas

**Date**: 2026-05-30  
**Status**: Accepted

## Summary

This ADR captures the current state and future direction of performance optimization work in TyphooN Terminal.

## Key Decision (from ADR-104)

We have decided **not** to pursue fully async separate task dispatch for multi-output indicators (MACD, Fisher, Ichimoku, etc.) at this stage. The engineering cost and risk outweigh the expected gains.

## Current Focus Areas

### 1. Widening the Forming-Bar O(1) Fast Path
- Currently only SMA and EMA have explicit last-value mutation support.
- Goal: Extend safe O(1) last-value mutation to as many indicators as possible without compromising correctness.

### 2. Allocation Reduction and Prioritization Improvements
- Reusable upload buffers for the full GPU path (implemented).
- 3-path GPU prioritization pattern (dedicated method → generic dispatch → CPU fallback) applied to eligible indicators.
- Further opportunities in hot paths will continue to be addressed.

## Scope and Constraints

All performance work must:
- Remain warning-free under `release-max` (full LTO).
- Not degrade visible UI or live forming-bar behavior.
- Prefer small, verifiable, incremental changes.

## Next Steps

- Evaluate additional indicators for safe forming-bar O(1) mutation.
- Continue systematic GPU prioritization for indicators supported by the `Indicator` enum and dispatch methods.
- Monitor for new allocation or prioritization opportunities after recent buffer reuse work.

## Related ADRs
- ADR-104: Async Multi-Output Indicator Dispatch Decision

This plan replaces the previous in-repository performance master plan.