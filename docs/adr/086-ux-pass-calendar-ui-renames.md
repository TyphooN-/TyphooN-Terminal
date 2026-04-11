# ADR-086 — UX Pass: Calendar UI, Staleness, Alerts, Help, Order Entry, Function Renames

**Status:** Implemented
**Date:** 2026-04-09

## Context

Follow-up to ADR-085. Remaining items from the UX audit and perf audit after the first wave landed. User asked to tackle the rest.

## Decisions

### 1. Economic Calendar UI parity with ForexFactory

The `econ_calendar.rs` scraper from ADR-085 returned all the data ForexFactory has (impact, forecast, previous, currency). The UI was rendering it in the old collapsed 5-tuple schema. This pass:

- **Added filter state** on the app struct: `econ_filter_high/medium/low/holiday` (impact checkboxes), `econ_filter_currencies` (comma-separated text: e.g. `USD,EUR,GBP`).
- **Added quick presets** — `USD` button and `Majors` button (`USD,EUR,GBP,JPY,CHF,CAD,AUD,NZD`).
- **Parsed the FF-flattened `actual` field** back into three columns: `Actual | Forecast | Previous`. The flattening (`fc:X (prev:Y)`) is undone at render time, so both Finnhub (which fills `Actual`) and ForexFactory (which fills `Forecast`/`Previous`) display correctly.
- **Color-coded impact column**: red (High), amber (Medium), green (Low), grey (Holiday).
- **Currency column** is cyan to draw the eye — traders filter by currency first.
- **Empty state**: actionable "No events loaded — click Refresh to fetch from ForexFactory (keyless)" instead of silent blank.
- **Staleness badge** in the header shows `updated Ns ago` / `updated Nm ago` / `updated Nh ago — STALE` with color escalation.
- **Source tag** `[ForexFactory]` vs `[Finnhub]` so the user knows which feed they're on.

### 2. Live panel staleness badges (Positions / Orders / Watchlist)

Added `positions_last_update_ts`, `orders_last_update_ts`, `watchlist_last_update_ts` on the app struct. Stamped in the respective `BrokerMsg` match arms (`Positions`, `Orders`, `WatchlistQuotes`).

New helper `staleness_badge(ts: i64) -> (String, Color32)` returns a relative-time label with an urgency color:
- Green `Ns` (< 30s)
- Grey `Ns` / `Nm` (< 10min)
- Amber `Nm` (< 10min warning)
- Red `Nm STALE` (> 10min)

Rendered inline in the right-panel collapsing headers: `Positions (4)  •  12s`, `Orders (0)  •  2m`, `Watchlist (9)  •  5m STALE`. Trader cannot exit on stale position data without seeing the warning.

### 3. Alert breach badge

Indicator alerts previously only logged to the console and optionally pinged Discord / Pushover / ntfy. If the alerts window was closed, there was no visible in-app cue.

Now:
- Three new state fields: `alert_breach_count: u32`, `alert_last_breach_ts: i64`, `alert_last_breach_msg: String`.
- When any `IndicatorAlert` fires, the count increments and the latest message is stashed.
- Top menu bar renders a prominent red button (`🔔 N ALERT`) when `count > 0`, with the latest message as tooltip text.
- Clicking the badge opens the alert builder window and clears the counter.

### 4. Help window overhaul

Previous help window was 20-row static grid, no search, no command reference. Now:
- **Searchable filter** at the top, matches both key and description (case-insensitive).
- **Three sections**: Chart navigation, App & window management, Command palette quick reference.
- **Top 31 commands** documented with one-line descriptions (SCOPE, OUTLIERS, DARWINVAR, EVENTS, CALENDAR, OPTION_CHAIN, FUNDAMENTALS, etc.).
- **GPU status footer** preserved (GPU indicators active / CPU fallback).
- Window resized to 720×560 to fit the content comfortably.

### 5. Order entry symbol autocomplete + validation

Previously: typing a bad symbol in the Order Entry dialog would silently fail at the broker with an error buried in the console log.

Now:
- Live validation against `self.all_broker_assets`. Known symbols render in green, unknown in red with hint `✗ unknown — broker will reject`. If no asset list is loaded (broker not connected), hints `(connect broker to validate)`.
- Prefix-match autocomplete: up to 10 suggestions from broker asset list first, then fallback to watchlist symbols. Each suggestion is a clickable selectable-label that fills the input.
- Symbol text edit uses monospace font at 160px wide for clarity.

### 6. Bad function renames (from audit)

Surveyed the codebase for badly-named functions. The codebase was mostly well-named — only 7 real offenders, of which 4 were worth renaming:

| Old | New | Why |
|-----|-----|-----|
| `round_price` | `format_order_price` | Returns a `String` with context-aware decimal precision, not a rounded `f64` |
| `yf_raw` | `yahoo_json_raw` | Cryptic abbreviation — now obvious it's Yahoo Finance JSON |
| `yf_fmt` | `yahoo_json_fmt` | Same |
| `extract_tail_timestamps` | `get_last_two_bar_timestamps` | "tail" was ambiguous — does it mean array end, file tail, trailing edge? |
| `parse_f64` (cli only) | `parse_csv_float` | Generic name shadowed across files; new name specifies the context (CSV row field) |
| `handle_ws` | `run_websocket_session` | Generic verb "handle" with no object — now describes the session lifecycle |

Names deliberately NOT renamed:
- `ui()` — egui trait override, idiomatic even though single-letter
- `new`, `from`, `on_bar`, `on_exit` — standard trait/callback patterns
- `get_account`, `get_positions` — clear REST-like naming
- `compute_*()` — pure calculations, correctly named

## Trade-offs

- **Autocomplete is prefix-match only**, not fuzzy. Good enough for tickers (users always type from the start) and cheaper than a fuzzy matcher.
- **Alert breach badge counts per-fire, not per-distinct-alert.** If the same alert re-fires (shouldn't, since `triggered` flag gates it) the count would grow. Acceptable — the counter is meant to be "stuff to look at", not a precise state machine.
- **Help window embeds command descriptions as a hand-maintained array.** Drifts from the actual `COMMANDS` registry over time. Acceptable since the top 31 commands are stable; the full registry is still discoverable via the command palette.
- **Staleness badges poll `chrono::Utc::now()` every frame.** Single syscall, cheap. Not worth throttling.

## Follow-ups

- Thread `broker_scope` through the remaining fundamental features (EV viewer, HV Cone, FUNDAMENTALS scraper).
- Persist `econ_filter_*` across sessions via `save_session()`.
- Alert badge should optionally play a system sound (requires a new audio crate dep).
- Help window command list should auto-generate from `COMMANDS` registry to avoid drift.

## Tests

All 619 tests pass (492 engine + 108 mql5-compiler + 14 web-protocol + 5 new cache tests from the prior commit). No new tests this pass — the changes are UI state and function renames, not new computation.

## Related

- ADR-082 — No-unwrap policy
- ADR-084 — Event Calendar + targeted outlier scanners
- ADR-085 — Broker scope filter + ForexFactory calendar + prior perf/UX pass
