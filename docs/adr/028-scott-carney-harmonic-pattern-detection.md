# ADR-028: Scott Carney Harmonic Pattern Detection

**Status:** Implemented
**Date:** 2026-03-24

## Context

Scott Carney's harmonic patterns (Gartley, Butterfly, Bat, Crab, Shark, Cypher, 5-0, Alt Bat, Deep Crab, Three Drives) are XABCD price structures defined by specific Fibonacci ratios between swing points. They provide high-probability reversal zones with predefined entries, targets, and stops — a complete trading strategy per pattern.

## Decision

Implement auto-scanning harmonic pattern detection in the native chart engine. Patterns are detected from Bill Williams fractals (5-bar swing points) and validated against Carney's Fibonacci ratio rules.

### Patterns Detected

| Pattern | AB/XA | BC/AB | XD/XA | Entry |
|---------|-------|-------|-------|-------|
| **Gartley** | 0.618 | 0.382-0.886 | 0.786 | D reversal |
| **Butterfly** | 0.786 | 0.382-0.886 | 1.27 | D reversal |
| **Bat** | 0.382-0.50 | 0.382-0.886 | 0.886 | D reversal |
| **Crab** | 0.382-0.618 | 0.382-0.886 | 1.618 | D reversal |
| **Shark** | 1.13-1.618 | 1.618-2.24 | 0.886 | D reversal |
| **Cypher** | 0.382-0.618 | 1.13-1.414 | 0.786 | D reversal |
| **5-0** | 1.13-1.618 | 1.618-2.24 | 0.50 BC | D reversal |
| **Alt Bat** | 0.382 | 0.382-0.886 | 1.13 | D reversal |
| **Deep Crab** | 0.886 | 0.382-0.886 | 1.618 | D reversal |
| **Three Drives** | 0.618-0.786 | 1.272-1.618 | — | Drive 3 |

### Detection Algorithm

1. Compute Bill Williams fractals (5-bar swing high/low)
2. Collect swing points from last 20 fractals
3. Test all XABCD combinations (alternating high/low)
4. Validate Fibonacci ratios against pattern rules (with tolerance)
5. Compute TP1 (0.382 AD), TP2 (0.618 AD), SL (beyond X)
6. Keep max 10 most recent patterns

### Rendering

- Cyan XABCD connecting lines with labeled vertices
- Green TP1/TP2 horizontal lines from D to chart edge
- Red SL horizontal line from D
- Pattern name + BULL/BEAR direction label

### Backtesting Potential

Harmonic patterns can be backtested across all symbols and timeframes:
- Entry at D completion
- TP1 at 0.382 AD retracement
- TP2 at 0.618 AD retracement
- SL beyond X point
- Can be combined with NNFX confirmation (Fisher, KAMA, volume)

This creates a systematic NNFX-style strategy: harmonic pattern for entry signal, NNFX indicators for confirmation.

## Consequences

- **Pro**: Complete entry/exit strategy per pattern (entry, TP1, TP2, SL)
- **Pro**: Works on any symbol and timeframe
- **Pro**: Can be backtested via existing backtest engine
- **Pro**: Combinable with NNFX confirmation indicators
- **Con**: O(n^5) pattern search on swing points — limited to last 20 swings for performance
- **Con**: Fibonacci ratio tolerances affect signal quality (too tight = miss patterns, too loose = false signals)
