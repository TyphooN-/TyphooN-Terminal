# ADR-110: Market Session Status Display (xStocks 24/5 + US Equities)

**Status:** Implemented with documented limitations | **Date:** 2026-06-08

## Context

The market-status chip in the toolbar was a binary OPEN/CLOSED label driven by
Alpaca's `/v2/clock` `is_open` flag. That flag is true **only** for the core
(regular) session, so during pre-market it read "US equities CLOSED · opens in
55m" even though the market is in pre-market with the core open ~55m out — Kraken
Pro shows "Open (Pre-market) · Core in …" for the same instant.

Kraken's tokenized stocks (xStocks) trade **24/5** — Sunday 8:00 PM ET →
Friday 8:00 PM ET — cycling four sessions each weekday: **overnight** (8 PM–4 AM,
Blue Ocean ATS), **pre-market** (4:00–9:30 ET), **core/regular** (9:30–16:00,
NASDAQ/NYSE), **after-hours** (16:00–20:00). A subset of ~10 popular xStocks
(TSLAx, SPYx, NVDAx, AAPLx, GOOGLx, QQQx, CRCLx, HOODx, MSTRx, GLDx) trade
**24/7** including weekends; most others are 24/5 and closed on US market
holidays.

This schedule is **published and deterministic** (Kraken support/docs), so the
session label does not require a live API. We separately asked whether Kraken's
API exposes the per-symbol schedule directly:

- **Public documented API** (`docs.kraken.com`): no. It is Spot / WebSocket /
  Futures / FIX (crypto). No equity/xStock trading hours, calendar, holidays, or
  session status, and no tokenized-stocks REST surface.
- **Internal iapi** (`iapi.kraken.com/api/internal`, already used for xStock
  prices/history): partial, and the useful part is already fetched. The catalog
  (`/markets/all/equities`) carries per-symbol **`overnight_trading_support`**
  (`enabled`/`disabled`), `vendor_attributes` (incl. `fractional_extended_hours_
  enabled`), and `has_tokenized`. The ticker carries `ext_trd_hrs`. The **full
  schedule panel** (weekly grid, holidays, listing exchange, timezone, "Core
  trading hours start in") is served by an undocumented iapi endpoint that could
  not be found by path-guessing (~26 candidates returned `Unknown method`); it
  must be captured from the Kraken Pro frontend (browser DevTools → Network).

## Decision

Compute the session label from the fixed ET schedule, with two distinct
calculators, and select per active chart:

1. **`kraken_xstocks_session_status_at(now, overnight_enabled)`** — the 24/5
   xStocks cycle (overnight / pre / core / after), weekend-closed Fri 8 PM →
   Sun 8 PM ET. Used for Kraken-equity charts.
2. **`us_equities_session_status_at(now, is_open, next_open, next_close)`** — the
   regular US market, which has **no overnight session**: pre-market → core →
   after-hours → CLOSED (8 PM–4 AM, weekends, holidays). Used as the global
   fallback when the active chart isn't an xStock.

Data sources for correctness:

- **Eastern time** comes from `us_eastern_offset_seconds` (DST-aware: 2nd Sunday
  March → 1st Sunday November), not a tz database.
- **US-equities holiday/half-day correctness** comes from Alpaca's `is_open` +
  `next_open` (the trading-day gate), with pre/after overlaid from fixed ET
  boundaries. A holiday mid-morning correctly reads CLOSED, not PRE-MARKET.
- **Per-symbol overnight** comes from the catalog `overnight_trading_support`
  flag we already fetch: `Some(false)` symbols are CLOSED 8 PM–4 AM (pre/core/
  after only); unknown/`Some(true)` get the full 24/5 cycle. Threaded as
  `KrakenEquityMarket.overnight_trading` → `kraken_equity_no_overnight` set →
  the session calculator.

## Implemented (2026-06-08)

- Pre-market fix: `us_equities_session_status_at` replaces the binary label.
- Per-symbol overnight: `overnight_trading` plumbed engine→native; the toolbar
  passes `overnight_enabled` per symbol.

## Implemented (2026-07-03) — interim US-market holiday awareness

The "small hard-coded NYSE/NASDAQ holiday table" interim option shipped, as a
**rule-based** calendar rather than a maintained year list (no annual upkeep):
`typhoon_engine::core::market_session::us_market_holiday` computes New Year's
(with the Dec-31 observed case), MLK Day, Washington's Birthday, Good Friday
(Easter computus), Memorial Day, Juneteenth (≥2022), Independence Day, Labor
Day, Thanksgiving, and Christmas, applying the exchange Sat→Fri / Sun→Mon
observation rule; `is_us_market_trading_day` combines it with the weekday
check. The xStocks calculator (`kraken_xstocks_session_status_at`) now:

- labels a holiday ET date `CLOSED · US market holiday (<name>)` instead of a
  normal session, opening at the next trading day's 04:00 ET pre-market;
- skips the Sunday 20:00 ET weekend open when Monday is a holiday (e.g. Labor
  Day weekend opens Tuesday pre-market);
- refuses to promise an overnight session into a holiday (Thursday evening
  before Good Friday reads CLOSED; Thursday after-hours counts down to the
  8 PM close). Conservative by design — prefer CLOSED over falsely OPEN.

Unit-tested in engine (`us_market_holidays_cover_fixed_observed_and_rule_based_dates`)
and native (`kraken_xstocks_session_status_is_holiday_aware`). Early-close
**half days** (Jul 3 as a trading half-day year-dependent, day after
Thanksgiving, Christmas Eve) are deliberately not modeled — still below.

## Deferred — needs the real iapi schedule endpoint (plan later)

Capture the undocumented per-symbol schedule endpoint from Kraken Pro DevTools,
then wire it (cached, low-frequency) to provide:

1. ~~**Authoritative xStock holidays.**~~ Interim rule-based holiday table
   shipped 2026-07-03 (above); the endpoint would still be more authoritative
   for exchange-specific one-off closures (e.g. mourning days).
2. **24/7 tier.** The ~10 weekend-trading symbols are not distinguishable from
   the catalog flags, so they still show weekend-CLOSED. Needs the endpoint (or a
   maintained list) to flag them.
3. **Exact half-day / early-close times** (e.g. 1 PM ET closes), beyond what
   Alpaca's `next_close` already gives the US-equities label.

## Regression guards

- Do **not** regress the market chip to a binary OPEN/CLOSED — pre/after/overnight
  are distinct sessions, not "closed."
- Keep the two calculators distinct: xStocks has an overnight session, the
  regular US market does not.
- Per-symbol overnight must come from the catalog `overnight_trading_support`
  flag, not be assumed uniform; unknown defaults to overnight-enabled.
- ET conversion stays in `us_eastern_offset_seconds` (DST-aware); don't hard-code
  a single UTC offset.

## Relationship to Other ADRs

- ADR-101 (Kraken iapi AIMD limiter) and ADR-102 (xStocks gap-fill) cover the
  same iapi surface; this ADR adds the trading-session view of it.
- ADR-057 (Yahoo extended-hours watchlist) is the related pre/post-market quote
  work.

## Sources

- Kraken xStocks: <https://www.kraken.com/xstocks>
- Market hours explained: <https://support.kraken.com/articles/market-hours-explained>
- 24/7 on Kraken Pro: <https://blog.kraken.com/product/xstocks/24-7-on-kraken-pro>
