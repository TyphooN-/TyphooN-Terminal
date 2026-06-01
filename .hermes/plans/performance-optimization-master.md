# TyphooN Terminal Performance Optimization Master Plan

Last updated: 2026-05-30
Status: Future work phase — Forming-bar O(1) fast path in progress

## Focus Area 1: Widening the Forming-Bar O(1) Fast Path

**Completed**:
- SMA (200/100)
- EMA (21)
- Disparity (using SMA100)
- CMO (with running sum_up/sum_down)
- Linear Regression Slope (with running sums)
- Momentum (simple approximate O(1))
- Rate of Change (simple approximate O(1))
- Linear Regression Intercept (using slope)
- Linear Regression Angle (atan(slope))
- Linear Regression (endpoint value)

**Next**:
- Evaluate whether further medium-risk indicators are worth the state
- Consider accuracy improvements or ring-buffer approach for approximate implementations

## Focus Area 2: Allocation Reduction & Prioritization
- Reusable upload buffers (done)
- 3-path GPU prioritization (multiple indicators done)

Continuing with forming-bar O(1) expansion.