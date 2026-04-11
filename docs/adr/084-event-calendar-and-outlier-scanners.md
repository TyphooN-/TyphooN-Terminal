# ADR-084 — Event Calendar & Targeted Outlier Scanners

**Status:** Accepted
**Date:** 2026-04-09

## Context

Three related needs surfaced during terminal use:

1. **Extended-hours candle regression.** The magenta pre/post-market candle added in ADR-078 never appeared on charts. Two bugs combined to suppress it silently.
2. **Targeted outlier scanning.** The existing `DARWINEXOUTLIERS` / `OUTLIERS` command runs a combined multi-dimensional scan (P/E + EV + short ratio + SEC activity). There was no way to look at a single dimension in isolation — specifically DARWIN VaR values (for corridor compliance) or enterprise value (for valuation anomalies).
3. **Upcoming events.** Dividend and earnings dates are stored in fundamentals but there was no consolidated "what's coming up" view filtered to the symbols the user is actually trading across brokers.

## Decisions

### 1. Extended-hours candle — bug fixes

**Bug A: geometry.** The guard `if next_x < chart_rect.right() - 10.0` always failed because:

```
bar_w  = chart_rect.width() / n_bars
next_x = chart_rect.left() + (n_bars + 0.5) * bar_w
       = chart_rect.right() + 0.5 * bar_w
```

`next_x` is always past the right edge. Both the ext candle AND the ghost placeholder were affected — neither ever rendered.

**Fix:** clamp `next_x` to `chart_rect.right() - half_body - 2.0`. This keeps the ext candle flush against the right edge without shifting the layout of regular bars (no change to `bar_w`).

**Bug B: wrong ext price math.** The WatchlistQuotes handler computed:
```rust
let ext_price = row.prev_close * (1.0 + row.ext_change_pct / 100.0);
```
But `ext_change_pct` is measured against intraday `regularMarketPrice`, not `previousClose`. The computation produced garbage.

**Fix:** use `row.last` directly. The Yahoo enrichment block (`GetWatchlistQuotes` handler) already overwrites `row.last` with the extended-hours price when ext data is present.

### 2. Targeted outlier commands

Two new console commands reuse the existing `var::detect_outliers` IQR engine and the existing outlier window, but narrow the data to a single dimension:

#### `DARWINVAR` / `VAROUTLIERS`
- Source: `self.bg.per_darwin_var` → `var_95` per DARWIN
- All DARWINs are placed in a single pseudo-sector (`"DARWIN"`) since they don't have fundamentals sectors
- Additionally checks Darwinex VaR corridor: **3.25%–6.5% of equity**
  - Below corridor → logged as warning
  - Above corridor → logged as error (rule violation)
- Requires ≥4 DARWINs with VaR data

#### `EVOUTLIERS` / `EV_OUTLIERS`
- Source: `fundamentals.enterprise_value`
- Grouped by sector (reusing existing IQR-by-sector logic)
- Requires ≥10 symbols with `enterprise_value > 0`

Both populate `darwinex_outliers` + `darwinex_sector_stats` and open the existing outlier window. `darwinex_multi_outliers` is cleared to avoid confusion.

### 3. Event Calendar

A comprehensive upcoming-events view, accessible via `EVENTS` / `EVENTCALENDAR` / `DIVEXPLORER`.

**Data model:**
```rust
enum EventSource { All, Alpaca, Darwinex, Tasty }
enum EventKind   { Earnings, ExDividend, DividendPayment }

struct EventRow {
    symbol, company, date, days_until, kind, detail,
    in_alpaca, in_darwinex, in_tasty,  // broker tradeability flags
}
```

**Source definitions:**
- **Alpaca active:** symbols in `self.live_positions` (open positions)
- **Tasty active:** symbols in `self.tt_positions`
- **Darwinex active:** symbols in `darwinex_radar_data` with `trade_mode != 0`, with Darwinex suffixes (`.US`, `.UK`, `.DE`, etc.) stripped to get the bare ticker

**Event extraction:** for each symbol present in ≥1 source, scan its `Fundamentals` for:
- `next_earnings_date` → `Earnings` (detail: P/E)
- `next_ex_dividend_date` → `ExDividend` (detail: dividend yield)
- `next_dividend_payment_date` → `DividendPayment` (detail: dividend yield)

Only events with `days_until ≥ 0` are kept. Rows are sorted ascending by `days_until` so the most imminent event is first.

**UI:** egui window with two filter rows:
- Source radio: All / Alpaca / Darwinex / Tasty
- Type checkboxes: Earnings / Ex-Div / Div Pay

Grid columns: Date, Days, Type (color-coded), Symbol, Company, Detail, Brokers (compact `ADT` tag string).

Date color: red (≤3 days), amber (≤7 days), muted otherwise — draws the eye to imminent events.

**`DIVEXPLORER` preset:** same command handler, but pre-selects `EventSource::Darwinex` and hides Earnings rows. It's a convenience entry point for "upcoming dividend dates for Darwinex-tradeable symbols", which was the original narrow request that expanded into the general Event Calendar.

## Alternatives considered

- **Reserve a right-edge slot in bar_w** for the ext candle (i.e. `bar_w = width / (n_bars + 1)`). Rejected — shifts the entire chart layout whenever ext state toggles, causing visible flicker. The clamp approach keeps regular bar positions stable.
- **Store an independent dividend explorer window** rather than folding it into the Event Calendar. Rejected — redundant state/UI for a strict subset of the calendar's data. A preset on the calendar command gives the same UX with no duplication.
- ~~**Fetch events from an external calendar API**~~ **Implemented in ADR-085.** Finnhub economic calendar with ForexFactory XML fallback, impact/currency filters.
- ~~**Include economic calendar events** (FOMC, CPI, NFP)~~ **Implemented in ADR-085.** Separate Economic Calendar window with country/impact/date columns.

## Consequences

**Positive:**
- Ext-hours candle now actually renders on US equity charts when Yahoo returns pre/post-market data.
- DARWIN corridor compliance check is one command away (`DARWINVAR`).
- Valuation anomaly hunting has a dedicated `EVOUTLIERS` command instead of having to run the full multi-dim scan.
- Event Calendar gives a single pane of glass for "what's coming up in the next N days for the symbols I care about."

**Trade-offs:**
- Ext candle clamp means the candle is cosmetically attached to the right edge rather than offset by a full bar. Acceptable — matches TradingView's "last bar hugs the right" behavior.
- ~~Event Calendar is read-only — no calendar ICS export yet.~~ **Implemented.** `EXPORT_CALENDAR` command writes `.ics` file via `build_events_ics()`. Source/impact/type filters respected.
- Darwinex "active" means "tradeable universe" (~6K symbols), not "currently held". The app doesn't know about DARWIN-internal positions held by underlying investors in the strategy. This is the best available proxy.

## Related

- ADR-078 — Yahoo extended-hours pipeline (original magenta candle)
- ADR-076 — Darwinex Radar (source of Darwinex tradeable universe)
- ADR-083 — Analytics expansion (multi-dim outlier scanner, dividend screener)
