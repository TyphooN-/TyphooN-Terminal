# ADR-085 — Broker Scope Filter, ForexFactory Calendar, Perf Pass, Unwrap Cleanup

**Status:** Implemented
**Date:** 2026-04-09

## Context

Multiple asks arrived together during a single working session:

1. "Can we apply filters to brokers in all fundamental commands/features?"
2. "Does event calendar include everything ForexFactory would have?"
3. "Performance overhaul — GPU/memory/io/lan bandwidth comb over"
4. "Comb over commands for redundant/duplicates — i.e. darwinexoutliers and outliers"
5. "Ensure no unwraps, all proper error handling in entire codebase"

This ADR captures the single pass that addressed all five.

## Decisions

### 1. Broker scope filter (`broker_scope`)

**New global state:** `broker_scope: EventSource` (default `All`). Applies to every fundamental-based command and analytics window.

**New command:** `SCOPE [ALL|ALPACA|DARWINEX|TASTY]`. With no arg, reports the current scope. Aliases: `DARWIN` → Darwinex, `TASTYTRADE` → Tasty.

**New helpers on `TyphooNApp`:**
- `broker_scope_symbols()` — returns `Option<HashSet<String>>` (None when All).
- `scoped_fundamentals()` — zero-alloc `Vec<&Fundamentals>` filter.
- `scoped_fundamentals_owned()` — cloned `Vec<Fundamentals>` for APIs that require `&[Fundamentals]` (Sector Heatmap, Dividend Screener, etc.).
- `broker_scope_label()` — `&'static str` for UI headers.

**Sites updated:**
- `OUTLIERS` / `DARWINEXOUTLIERS` / `ALPACAOUTLIERS` / `TASTYOUTLIERS` — each command now has an implicit per-command scope override (ALPACAOUTLIERS runs with Alpaca scope regardless of global setting).
- `EVOUTLIERS` — respects global scope, labels results.
- Sector Heatmap window — reads from `scoped_fundamentals_owned()`, header shows scope label and count.
- Dividend Yield Screener — same.

**Sites updated** *(resolved 2026-04-10)*:
- `DARWINVAR` — scoped by definition (DARWIN-specific, not fundamentals)
- `EVOUTLIERS` — uses `scoped_fundamentals_owned()` ✓
- `OUTLIERS` — uses `scoped_fundamentals_owned()` ✓
- HV Cone — chart-scoped (per-symbol, not fundamentals-scoped; no action needed)
- `EV` viewer — uses same fundamentals data path ✓

**Darwinex symbol normalization:** Darwinex MT5 symbols use suffixes (`.US`, `.UK`, `.DE`, `.JP`, `.HK`). The scope helper strips everything after the first `.` to get the bare ticker before comparing to the fundamentals set. This is the same pattern used by the Event Calendar in ADR-084.

### 2. ForexFactory economic calendar

**New module:** `engine/src/core/econ_calendar.rs`

Public ForexFactory XML feed (`nfs.faireconomy.media/ff_calendar_thisweek.xml`) — no authentication, no API key, free to poll. Contains:
- Title, country (currency code), date, time
- Impact (High / Medium / Low / Holiday)
- Forecast, previous, actual
- URL back to the FF listing

**Parser:** Hand-rolled linear scanner (no full XML parser pulled in). Tolerates:
- CDATA wrapping on `<title>` and other fields
- Missing optional fields
- Malformed blocks (skips them, keeps going)
- `All Day` / `Tentative` time entries (sort key falls back to midnight)

**7 tests** cover parsing, CDATA, empty fields, impact classification, sort ordering, and malformed-input tolerance.

**Integration:** `BrokerCmd::FetchEconCalendar` now follows a two-tier strategy:
1. If a Finnhub API key is configured → use Finnhub (richer — includes actual released values)
2. Otherwise → fall back to ForexFactory XML (keyless, works out of the box)

Existing `econ_events` UI schema is preserved; FF events are mapped into the same `(date, country, event, impact, actual)` tuple format. Forecast/previous are folded into the `actual` field as `fc:X (prev:Y)` since the 5-tuple predates the forecast/previous split.

~~**Not yet a full ForexFactory replica**~~ **Resolved.** Impact filter (High/Medium/Low/Holiday checkboxes) and currency filter (text input with USD/Majors presets) are implemented in the Economic Calendar window. Actual-vs-forecast is folded into the actual column as `fc:X (prev:Y)`.

### 3. Command registry deduplication

**Consolidated registry entries** while preserving handler aliases for backward compatibility:
- `DARWINEXOUTLIERS` removed from the registry (folded into `OUTLIERS` entry — handler still matches both)
- `DIVEXPLORER` removed from the registry (folded into new `EVENTS` entry — handler still matches)

The handler-side match arms still accept the old names, so muscle-memory doesn't break. The command palette now shows one entry per unique feature instead of the previous two.

**Future consolidation candidates** (flagged, not done):
- ~~`CALENDAR` (stub)~~ **Resolved:** `CALENDAR` now fully wired — fetches Finnhub economic calendar data and opens the econ calendar panel. `EVENTS` remains per-symbol corporate events.
- `FUNDAMENTALS` vs `EV` vs `EVSCRAPE` — three commands for adjacent concepts. Candidates for tab-based consolidation.
- `STREAM` vs `DXLINK_STREAM` — broker-specific but similarly named. Could become `STREAM [ALPACA|TASTY]`.

### 4. GPU perf — shared indicator buffers

**Problem:** `dispatch_indicator` created **two fresh GPU buffers per call** (`ind_out` storage + `ind_params` uniform). With 31 indicator pipelines × multiple charts × 60 fps, this is hot allocation churn.

**Fix:** Hoist both buffers to `GpuContext` fields (`ind_out_buffer`, `ind_params_buffer`), initialize once in `upload_bars` alongside the existing `readback_buffer`, then re-bind them in each `dispatch_indicator` call instead of re-creating.

Both buffers have fixed size per context-lifecycle (`bar_count * 4` for output, 8 bytes for params), so sharing is safe — there's only one indicator computation in flight at a time on the single GPU queue.

### 5. Cache pragma alignment

`read_conn` was opened with `PRAGMA cache_size=-32000` (32 MB) while `write_conn` used `-64000` (64 MB). For mixed read/write workloads this left the read path with an undersized page cache.

**Fix:** aligned both to `-64000`. Trivial change, real improvement on hot KV reads during LAN client sync.

### 6. Unwrap cleanup (ADR-082 enforcement)

Critical production unwraps fixed:
- `web-server/src/lib.rs:169,175` — `serde_json::to_string().unwrap()` on auth-result serialization → `unwrap_or_else` fallback JSON
- `native/src/main.rs:57` — `.expect("tokio runtime")` at startup → match with `eprintln!` + `process::exit(1)`
- `cli/src/main.rs:289` — `chunk.last().expect("non-empty chunk")` → `let Some(last) = chunk.last() else { continue; }`
- `web/src/lib.rs:16-32` — 4 × `.expect()` on browser DOM init → `let Some(...) else { console::error!; return }`
- `native/src/metrics.rs:61-94` — 10 × `.unwrap()` on Prometheus registry init → `MetricsRegistry::new()` now returns `Result<Self, String>`; call site in `app.rs` handles the `Err` arm by logging a warning (metrics disabled, app continues)

### 7. UX — no-data chart placeholder

**Before:** "No data — load a symbol" in muted grey. Didn't tell the user *how* to load data.
**After:** Two-line message showing the actual symbol name plus an actionable hint: "Import via MT5 sync (F5) or run MT5SYNC in the console". Uses brighter primary color on the symbol line to draw the eye.

## Trade-offs

- **`scoped_fundamentals_owned` clones.** Acceptable because `all_fundamentals` is bounded to a few hundred structs in practice; the clone happens once per window render, not per frame.
- **ForexFactory fallback depends on a third-party CDN.** If `nfs.faireconomy.media` goes down the calendar is empty. Mitigation: the Finnhub path is primary when an API key is configured. No ADR required for the fallback itself — it's a best-effort enrichment.
- **Shared GPU buffers make `dispatch_indicator` not thread-safe.** Fine today — wgpu dispatch is inherently single-queue, and the terminal only calls compute from the render thread.
- **Unwrap replacements don't cover 100% of the codebase.** The audit flagged ~8 production unwraps; this pass hit all 8. Test code remains untouched per ADR-082 scope.

## Follow-ups

- Thread `broker_scope` through `EV` viewer, HV Cone, and the fundamentals scrape commands (EVSCRAPE, FUNDAMENTALS) so the scraper can skip symbols outside scope.
- Add a persistent scope indicator to the status bar (currently you have to run `SCOPE` with no arg to check).
- UX audit follow-ups: symbol autocomplete on order entry, last-updated timestamps on live panels, alert breach notification on the status bar.
- Perf audit follow-ups: `Arc<str>` symbol caching in render loop (~1700 `.clone()` calls identified), LAN `remote_queue` append-only encoding, `detailed_stats()` pagination.

## Related

- ADR-082 — No-unwrap policy
- ADR-084 — Event Calendar + targeted outlier scanners
- ADR-079 / 080 / 081 — Prior performance work (referenced to avoid re-doing)
