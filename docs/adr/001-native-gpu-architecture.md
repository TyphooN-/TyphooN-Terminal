# ADR-001: Native GPU Architecture

**Status:** Implemented
**Date:** 2026-03-24

## Context

Trading terminals built on Electron/WebKit suffer from high memory overhead, GC pauses, and a large attack surface. A professional trading terminal requires deterministic frame timing, minimal input latency, and direct GPU access for real-time chart rendering.

## Decision

Build the terminal as a pure Rust application using egui for the immediate-mode UI and wgpu for GPU-accelerated rendering. Zero JavaScript, zero WebKit. The data path is SQLite cache -> Rust structs -> GPU vertex buffers with no serialization boundaries. Broker communication runs on a tokio async runtime, with the UI thread receiving updates through bounded mpsc channels (BrokerCmd outbound, BrokerMsg inbound). The main loop is single-threaded egui with repaint-on-event to keep idle CPU near zero.

## Consequences

- Direct memory path from SQLite to GPU eliminates JS bridge overhead and GC pauses
- wgpu provides Vulkan/Metal/DX12 backends; runs on Linux, macOS, Windows without code changes
- Immediate-mode UI means zero retained widget state bugs; every frame is a fresh layout pass
- mpsc channels decouple broker latency from render latency; UI never blocks on network
- No WebView means no DOM, no CSS layout engine, no browser security surface to maintain
- Trade-off: no HTML/CSS ecosystem; all UI widgets must be built or sourced from egui crates
- Compile times are longer than JS but produce a single static binary with no runtime dependencies
