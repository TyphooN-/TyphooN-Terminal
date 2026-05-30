# ADR-104: Async Multi-Output Indicator Dispatch

**Date**: 2026-05-30  
**Status**: Accepted

## Context

We evaluated whether to pursue a more aggressive async model for multi-output GPU indicators (MACD, Fisher, Ichimoku, etc.) by running portions of their computation in separate async tasks, leveraging the highly asynchronous nature of the GPU.

The goal was to potentially improve frame-time utilization when many complex indicators are active.

## Decision

We will **not** pursue fully async separate task dispatch for multi-output indicators at this time.

### Rationale

- The engineering cost is high. It would require significant changes to `GpuCompute`, command buffer management, and synchronization.
- Most indicators are already very fast once data is resident on the GPU. The dominant costs are upload/readback and forming-bar handling, not compute parallelism.
- Dedicated methods for complex indicators (e.g. `compute_macd_gpu_dynamic`, `compute_fisher_gpu`) are already well-optimized.
- The risk of introducing subtle correctness bugs (especially around forming-bar updates) outweighs the expected gains.
- Higher-leverage performance work exists in other areas (forming-bar O(1) paths, allocation reduction, prioritization).

## Consequences

- We keep the current architecture: dedicated methods for complex/multi-output indicators + the 3-path prioritization pattern where applicable.
- Future work on async dispatch is not ruled out, but will only be reconsidered if profiling shows it as a clear bottleneck.
- Focus shifts to widening forming-bar O(1) fast paths and continuing allocation/prioritization improvements.

## References

- Performance Master Plan (`.hermes/plans/performance-optimization-master.md`)
- Item 1 (GPU prioritization) and OHLC dispatch abstraction work