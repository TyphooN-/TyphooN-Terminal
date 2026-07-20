# ADR-134: Render-Independent Background Pump (Hidden-Window Sync)

**Status:** Accepted / Implemented (2026-07-14)
**Date:** 2026-07-14

## Context

The entire data pump — bar-sync scheduling, the broker-message drain
(ingest bookkeeping, backfill/no-data marks, pending-fetch settlement), the
Alpaca rotation dispatch, WS status handling, retry queues, metrics, and
session persistence — lived inside the egui frame body. eframe only runs the
frame body when the compositor requests frames, and Wayland compositors
(Hyprland included) withhold frame callbacks from windows on inactive
workspaces. winit gates `RedrawRequested` delivery on those callbacks while
`window.is_visible()` still reports `true`, so neither eframe's
invisible-window direct-paint path (egui #5229) nor any repaint request ever
runs the frame again.

Observed overnight (2026-07-12 → 2026-07-13): with the window on another
workspace, only pure tokio tasks (`app_background`: halts refresh, LRU
eviction, news retention) kept logging. The unbounded `BrokerMsg` channel
accumulated results, `pending_fetches` stayed wedged at ~256 so no new work
was dispatched, Alpaca backfill made zero progress for ~9.5 hours, Kraken WS
reconnect/status messages queued unprocessed, and refocusing triggered a
burst drain (575+ messages, multi-hundred-ms frames, CPU spike across all
cores). Bar-sync health decayed to 2.9-16.6% on intraday lanes.

Two structural facts drove the fix:

1. eframe 0.35 already splits the App contract: `App::logic` is documented to
   run "also when the UI is hidden, but `request_repaint` was called", and
   `epi_integration::update` runs `logic()` on every pass while gating
   `ui()` + tessellate/paint/present behind viewport visibility. The app
   deliberately kept one body (pre-split comment in `app_runtime.rs`); that
   choice is what this ADR reverses.
2. Upstream eframe never reaches that logic-only path on Wayland-occluded
   windows: `check_redraw_requests` calls `window.request_redraw()` (starves
   forever) because the window is not `is_invisible_or_minimized`.

## Decision

### 1. Vendored eframe 0.35.0 with a redraw-starvation watchdog

`vendor/eframe` (wired via `[patch.crates-io]`; workspace `exclude`;
attribution in `NOTICE`; all hunks marked "TyphooN-Terminal local patch",
re-apply procedure in `vendor/eframe/README.md`):

- `run.rs` tracks, per window, the oldest `request_redraw()` with no
  `RedrawRequested` delivered since (`windows_undelivered_redraw_since`).
  Once undelivered for ≥1s (`REDRAW_STARVATION_THRESHOLD`), due repaints stop
  going through `request_redraw()` and instead directly invoke
  `run_ui_and_paint(.., background_tick = true)`, clamped at tick time to one
  per 250ms (`STARVED_WINDOW_TICK_INTERVAL`). Deadline entries are re-armed
  instead of dropped, so the watchdog stays alive even if the app stops
  requesting repaints, and one redraw request is kept outstanding so painting
  resumes the instant the compositor serves the window again. A genuinely
  delivered `RedrawRequested` clears the starvation state.
- `wgpu_integration.rs` / `glow_integration.rs`: `background_tick = true`
  forces the pass to treat the viewport as invisible — `App::logic` runs,
  `App::ui`, tessellation, paint, and present are all skipped. Presenting to
  an occluded Wayland swapchain could otherwise block the event-loop thread
  in acquire, which would be strictly worse than the original bug.

Live-verified on Hyprland (2026-07-14) with a smoke app: visible = logic+ui
in lockstep; moved to a hidden workspace = ui frozen, logic ticking ~2-4/s;
moved back = painting resumes immediately.

### 2. Frame body split: `App::logic` = pump, `App::ui` = render

`typhoon-native/src/app/app_runtime.rs`:

- `logic()` now owns every sync-critical tick: the full pre-broker tick list
  (retry queue, state caches, kraken catalog/universe/WS schedulers, bar-sync
  status refresh, cache startup, background-snapshot drain, deferred chart
  loads, indicator recompute, chart background results),
  `tick_broker_messages`, the credential safety-net, Prometheus metrics, the
  weekend crypto rotation, the Alpaca rotation dispatch
  (`schedule_alpaca_pairs`), and `maybe_incremental_session_save`. Relative
  order (ticks → drain → dispatch) is preserved from the old single body.
- `ui()` keeps rendering only: style init, chrome panels, floating windows,
  central panel, console, screenshot capture, frame-stall telemetry, and the
  visible repaint policy. Log-line field names are unchanged; pump-side
  timings cross the split via `pump_*` fields on the app state.
- Every `logic()` pass arms `request_repaint_after(1s)` as a wakeup floor
  (min-merged, so it never slows visible rendering). The drain's existing
  cap-hit `request_repaint_after(16ms)` gives hidden catch-up at the 250ms
  watchdog clamp.
- Hidden mode is detected by `last_ui_frame_at` aging past 2s. While hidden:
  the broker drain budget rises from 16-48 msgs / 3-8ms to 512 msgs / 50ms
  (no frame pacing to protect), an INFO heartbeat logs every 5 minutes
  ("background pump heartbeat (window hidden): …"), and entry/resume are
  logged so overnight liveness is provable from the log alone.
- The weekend crypto rotation index moved from `frame_count / 240` (frozen
  whenever rendering stops) to a per-tick round-robin cursor.

## Consequences

- Bar sync, ingest, dispatch, WS status handling, session saves, and metrics
  continue at full speed with the window on another workspace, occluded, or
  minimized — sync no longer depends on the user focusing the window, and
  refocus burst-drains (and their CPU spikes) disappear because the backlog
  never forms.
- Overnight logs now show pump heartbeats instead of silence; "window hidden"
  / "window visible again" lines bracket hidden periods.
- Hidden CPU floor is bounded: ≤4 logic ticks/s × pump cost, plus ≤200ms/s
  drain only while a backlog exists.
- Maintenance cost: eframe upgrades require re-vendoring and re-applying the
  marked hunks (see `vendor/eframe/README.md`). The patch is deliberately
  small (4 files) and behavior-neutral for healthy redraw delivery, X11,
  Windows, and macOS: the watchdog only engages after 1s of starved redraws.
- Screenshot capture stays `ui()`-only (needs painted frames). Chart-display
  ticks intentionally keep running while hidden so refocus is instant; if
  hidden CPU ever matters, they are the first candidates to gate.
