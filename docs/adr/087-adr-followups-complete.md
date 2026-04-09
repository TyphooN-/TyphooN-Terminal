# ADR-087 — Close Out Remaining ADR Follow-ups (Help auto-gen, Scope Indicator, Session Persistence, EV/Fund Scope, ICS Export, Alert Attention)

**Status:** Accepted
**Date:** 2026-04-09

## Context

ADRs 084, 085, 086 each ended with a "Follow-ups" section listing items deferred in favor of getting the core work landed. This ADR closes out all the concrete actionable items from those sections in a single pass.

## Decisions

### 1. Help window auto-generates from `COMMANDS` registry (ADR-086 follow-up)

Previously the help window had a hand-maintained array of ~31 command descriptions, flagged in ADR-086 as drift-prone.

**Fix:** iterate `COMMANDS` directly. Split into two sections:
- **Command palette** — all non-`DRAW_*` entries shown by default (count live-updates from the registry).
- **Drawing tools** — the ~50 `DRAW_*` entries, tucked behind a collapsible so the main list stays scannable.

Both sections respect the existing `help_filter` search box. Now when a new command is added to `COMMANDS`, the help window picks it up automatically with zero maintenance.

### 2. Persistent broker scope indicator in top bar (ADR-085 follow-up)

Previously: to check the current `broker_scope` you had to run `SCOPE` (no args) and read the log.

**Fix:** Click-to-cycle button in the top menu bar, right after the title tag. Color-coded per scope:
- `Scope: ALL` — muted grey
- `Scope: ALPACA` — orange
- `Scope: DARWINEX` — blue
- `Scope: TASTY` — purple

Clicking cycles through `All → Alpaca → Darwinex → Tasty → All` and logs the count of in-scope fundamentals. Tooltip explains what the filter affects.

### 3. Session persistence for scope + econ filters (ADR-086 follow-up)

Added six new fields to `build_session_value()` and the matching load path in `load_session()`:
- `broker_scope` — serialized as lowercase string (`"all"`, `"alpaca"`, `"darwinex"`, `"tasty"`)
- `econ_filter_high` / `econ_filter_medium` / `econ_filter_low` / `econ_filter_holiday` — bool
- `econ_filter_currencies` — string

Trader's filter preferences now persist across app restarts.

### 4. Broker scope threaded through EV viewer + EVSCRAPE scraper (ADR-085 follow-up)

**EV viewer:** added scope filter to the fundamentals selection chain. Header now shows `{n} symbols • scope: {LABEL}`. Combined with the existing "Active Only" filter so a trader can narrow further.

**EVSCRAPE command:** override `use_mt5`/`use_alpaca`/`use_tastytrade` flags based on the current `broker_scope`. When scope=`ALPACA` the scraper only hits Alpaca, etc. When scope=`ALL` (default) it uses the configured source toggles as before. Log line now includes the active scope label.

### 5. HV Cone scope — NOT applicable (ADR-085 item rejected)

The ADR-085 follow-up listed HV Cone for scope threading, but the HV Cone window is **chart-scoped**, not fundamentals-scoped: it computes historical volatility on the visible bars of the currently-active chart. `broker_scope` doesn't apply. Removed from the follow-up list.

### 6. Alert badge OS attention request (ADR-086 "optional system sound")

The follow-up suggested adding a sound crate dep (rodio or similar, ~25 crates) for audible alert cues.

**Fix taken instead:** use `ViewportCommand::RequestUserAttention(Critical)` which is already in egui 0.34. On Linux the taskbar icon flashes, on macOS the dock bounces, on Windows the title bar flashes. No new dep, cross-platform, and arguably better than audio (doesn't disturb meetings / shared spaces).

### 7. ICS export for Event Calendar (ADR-084 follow-up)

New `Self::build_events_ics()` builder emits an RFC 5545 iCalendar payload from the current filtered event list. New `Export .ics` button in the Event Calendar type-filter row writes the payload to `~/typhoon_events.ics`.

Implementation details:
- Events are emitted as all-day `VEVENTs` (`DTSTART;VALUE=DATE` / `DTEND;VALUE=DATE`) — we only store date strings, not precise times.
- Respects the active source + type filters at the moment of export.
- Proper RFC 5545 escaping for commas, semicolons, backslashes, newlines.
- Un-parseable dates are skipped gracefully.
- `UID` format `{symbol}-{date}-{kind}@typhoon-terminal` — stable across re-imports so Google/Apple Calendar updates rather than duplicates.

**6 unit tests** cover: calendar wrapper, VEVENT emission, source filter, type filter, special-char escaping, and unparseable-date skipping.

Output is compatible with Google Calendar, Apple Calendar, Outlook, Thunderbird.

## Trade-offs

- **ICS is read-only export, not two-way sync.** Deliberate — sync would require OAuth + polling. Import-once-per-update is the common 90% workflow.
- **Scope indicator cycles instead of opens a dropdown.** Simpler, one-click mental model. Dropdown would let you skip states but adds a menu widget for three states. Click-to-cycle wins for this count.
- **Session format is additive** — old session files missing the new fields just default them (handled by `as_str().unwrap_or(...)` paths). No migration script needed.
- **`RequestUserAttention(Critical)` may be no-op** on some window managers (minimal tiling WMs). Acceptable — the in-app red badge still fires.

## Deferred / Out of Scope

- **Calendar ICS live URL feed** (so calendars auto-refresh). Would need a small always-on HTTP server — out of scope for a local terminal.
- **Persistent help window expansion state.** Users can always re-collapse; minor.
- **`Arc<str>` symbol caching.** Already deferred in ADR-085 with justification — stands.

## Tests

- 6 new tests for `build_events_ics` (wrapper, count, source filter, type filter, escaping, bad-date skip)
- Total: 497 engine + 108 mql5 + 14 web-protocol + 78 native (up from 72) = **697 passing**

## Related

- ADR-084 — Event Calendar + outlier scanners (originated ICS export deferral)
- ADR-085 — Broker scope + ForexFactory (originated scope indicator, EV scope, fund scraper scope)
- ADR-086 — UX pass (originated help auto-gen, econ filter persistence, alert sound)
