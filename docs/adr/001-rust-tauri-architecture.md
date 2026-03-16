# ADR-001: Rust + Tauri Architecture

**Status:** Accepted
**Date:** 2026-03-15
**Context:** Need a local desktop trading terminal to replace MT5 for Alpaca Markets.

## Decision

Use Rust (Tauri 2.0) backend with lightweight-charts JavaScript frontend.

## Alternatives Rejected

- **Python**: No interactive charting, GIL bottleneck, two-process complexity, 200MB+ frozen bundles
- **Electron**: 150-200MB binary, 200-500MB RAM, Node.js not ideal for financial math
- **Qt/C++**: Slow development, no memory safety, expensive licensing, no charting library
- **Pure Rust GUI (egui/iced)**: Immature financial charting — deferred as future migration path

## Consequences

- ~10MB binary, <100MB RAM, <1s startup
- Risk engine in Rust with zero-cost abstractions and type safety
- Battle-tested charting via lightweight-charts (MIT, 170KB)
- Cross-platform via system webview (no bundled browser)

See [ARCHITECTURE.md](../ARCHITECTURE.md) for full rationale.
