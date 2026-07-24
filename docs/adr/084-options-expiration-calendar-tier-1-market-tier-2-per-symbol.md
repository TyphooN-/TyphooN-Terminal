# ADR-084: Options Expiration Calendar — Tier 1 Market + Tier 2 Per-Symbol

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-079 (Options chain fetch), ADR-079 (BBSQUEEZE)
**Related:** `typhoon-engine/src/core/research/`, `typhoon-native/src/app.rs`, `typhoon-engine/src/core/lan_sync.rs`

## Context

The terminal has shipped option chain fetching since ADR-079 (Yahoo
options endpoint → `OptionsChainSnapshot.expirations[]`), the
tastytrade chain integration, and an Option Chain egui window that
renders per-expiration strike grids with on-the-fly Greeks. What has
been missing is a **calendar-oriented view** that answers the two
most common options-analyst questions independent of strike selection:

1. **"What are the upcoming expiration dates for the market overall,
   and when is the next triple witching?"** — a calendar view that
   needs no data at all beyond date math.
2. **"For this specific symbol, what do the upcoming expirations look
   like in terms of call/put volume, open interest, and put/call
   ratio?"** — an aggregation layer over the existing chain.

The user approved building both tiers in a single feature (a directive
of "both" when asked to choose). Building both behind one window with
a tab selector gives a coherent UX: the user opens the calendar, sees
the next 90 days of market expirations by default, and can tab to the
per-symbol view to get chain-specific aggregates.

### Why a new surface rather than extending the Option Chain window

The Option Chain window is strike-oriented — each expiration opens a
collapsible that lists every strike with its Greeks. That layout is
right for "I want to see the $170 call for next Friday" and wrong for
"I want to compare put/call ratios across the next 12 expirations."
Mixing them would force a single window into two different mental
models. Separate window, separate question.

### Why classify expirations

Without classification, upcoming expirations are a flat list of
YYYY-MM-DD strings. With classification as WEEKLY / MONTHLY /
QUARTERLY / TRIPLE_WITCHING / LEAPS, the AI packet emitter can
highlight the next triple-witching date, and the user can see at a
glance which Friday is a monthly vs a weekly. The classification
rules are deterministic date math so the same logic powers both
Tier 1 (future dates with no chain) and Tier 2 (dates from an existing
chain).

## Decision

### Tier 1 — Offline market calendar

Pure date math, no DB, no API. Implemented as:

- `is_third_friday(date: &NaiveDate) -> bool` — true iff `weekday ==
  Fri && 15 <= day <= 21`.
- `is_triple_witching(date: &NaiveDate) -> bool` — true iff
  `is_third_friday && month ∈ {3, 6, 9, 12}`.
- `classify_expiration(date, reference_today) -> String` — returns
  one of `TRIPLE_WITCHING / LEAPS / QUARTERLY / MONTHLY / WEEKLY`.
  LEAPS threshold is "3rd Friday more than 270 days from reference";
  everything shorter that is a 3rd Friday in Mar/Jun/Sep/Dec that is
  not TW becomes QUARTERLY (third Friday in a quarter-end month
  outside the LEAPS horizon is always TW actually — kept the
  QUARTERLY arm for symmetry with rarer non-TW quarterly surfaces).
- `CalendarExpiry { date, weekday, days_from_now, expiry_type,
  is_triple_witching }` — one row per Friday.
- `compute_market_calendar(from, horizon_days) -> Vec<CalendarExpiry>` —
  walks `from`→`+horizon_days` day-by-day, emits every Friday.

### Tier 2 — Per-symbol chain snapshot

Reads the existing `research_options_chain` cache, aggregates per
expiration, and persists as a new `SymbolExpirationsSnapshot` in a
new `research_symbol_expirations` table (v56 schema). Structs:

- `SymbolExpiration { date, days_to_expiry, expiry_type, call_count,
  put_count, total_call_volume, total_put_volume, total_call_oi,
  total_put_oi, put_call_ratio }`.
- `SymbolExpirationsSnapshot { symbol, as_of, underlying_price,
  expirations, next_triple_witching, note }`.

`compute_symbol_expirations(conn, symbol)` reads the chain, parses
each `OptionExpiry`'s date, applies `classify_expiration`, sums
volume and open interest across strikes, computes put/call ratio,
and surfaces the nearest upcoming triple-witching date.

### Persistence

New `research_symbol_expirations` table (v56). Standard JSON-blob
pattern matching every other research surface:

```sql
CREATE TABLE IF NOT EXISTS research_symbol_expirations (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
```

Tier 1 is pure compute — **not persisted**. It regenerates on every
window open with the current date as the reference.

### LAN sync

`research_symbol_expirations` added to `SYNCABLE_TABLES` with the
standard CREATE TABLE stanza and `updated_at` timestamp column
entry.

### Native UI

Single egui window `EXPCAL` with two tabs:

- **Market calendar**: horizon-days slider (7–730), regenerate
  button, striped 4-column grid (Date / Weekday / DTE / Type) with
  type-color coding (TRIPLE_WITCHING in DOWN red, QUARTERLY/MONTHLY
  in UP green, LEAPS/WEEKLY in AXIS_TEXT).
- **Symbol chain**: standard Symbol textbox + Use-Chart + Load-Cached
  + Compute. On Compute, a tokio task runs
  `compute_symbol_expirations` off-broker and persists via
  `upsert_symbol_expirations`, then sends `SymbolExpirationsMsg`
  back. Grid shows 9 columns: Date / DTE / Type / Calls / Puts /
  Call Vol / Put Vol / Call OI / Put OI + PCR. Header highlights
  next triple-witching date in red.

### Palette aliases

`EXPCAL | OPTCAL | EXPIRY | EXPIRATIONS | OPTION_CALENDAR |
OPTIONS_CALENDAR | OPTION_EXPIRATION_CALENDAR`.

No collision with existing palette tokens. `CALENDAR` alone is
already claimed by the economic calendar window; `OPTIONS` by the
chain-fetch trigger.

### Packet emitter

Tier 2 emits into the research packet under `### Options Expiration
Calendar — EXPCAL`. Format: header line with count + next triple
witching, underlying price, then up to 12 expiration rows with
DTE / type / call/put counts / volumes / OI / PCR. Truncation note
if more than 12. Tier 1 does not emit — it is a UI-only convenience,
symbol-agnostic, and adding it to a per-symbol packet would waste
tokens on identical data for every symbol.

## Consequences

### Positive

- **Answers the two canonical calendar questions** in one window:
  "when's the next triple witching" and "what does this symbol's
  chain look like across expirations."
- **Zero new API dependencies.** Tier 1 is pure date math; Tier 2
  reuses the existing `research_options_chain` cache populated by
  the OPTIONS command.
- **Deterministic classification** — Tier 1 and Tier 2 share the
  same `classify_expiration` function, so the same date gets the
  same label whether the user is browsing the market calendar or
  inspecting a chain.
- **LAN-synced Tier 2** — a client that has computed symbol
  expirations will sync them to peers alongside every other
  research table; a fresh peer doesn't need to re-fetch the
  options chain if Tier 2 is already cached.
- **Packet emitter highlights next triple witching** as a single
  high-signal field the AI can reason over independently of the
  full expiration list.

### Negative / Risks

- **Tier 2 depends on a cached chain.** First-time users will see
  "No data — run OPTIONS first" until a chain has been fetched.
  This is acceptable (the chain fetch is one palette command away)
  but requires documentation.
- **Yahoo requires one request per expiration.** The chain fetcher now
  discovers `expirationDates` on the first request and hydrates each date
  with a bounded `date=` follow-up request. Partial provider failures are
  preserved in `OptionsChainSnapshot.note`; Tier 2 can classify across all
  successfully fetched expirations instead of being limited to the first
  row.
- **Quarterly classification rule is slightly redundant** with
  TRIPLE_WITCHING — in the current calendar, every 3rd Friday of
  a quarter-end month within the LEAPS horizon *is* TW, so the
  QUARTERLY arm only triggers in the unusual case where a non-TW
  quarterly happens. Kept the arm explicit for future
  extensibility (e.g. quarterly SPX expirations that aren't TW).
- ~~**No "max pain" strike calculation yet.**~~ **Shipped
  (2026-07-04):** `max_pain_strike` / `max_pain_by_expiration`
  (engine, OI-weighted intrinsic-payout minimization over the
  union of chain strikes, unit-tested) and the packet's Options
  Chain section now prints `Max pain (OI-weighted)` per cached
  expiration, so the AI reads it directly instead of eyeballing
  the highest-OI strike.

### Neutral

- The Option Chain window and the Expiration Calendar window are
  complementary, not competing. The user can open both
  simultaneously; one is strike-oriented, the other is
  date-oriented.
- `chrono` was already a workspace dependency; no new crates.

### Paid-API gap

None introduced. Yahoo options endpoint remains the free-tier chain
source. Paid endpoints such as OpEra or Polygon may still be useful for
faster refresh, greeks, and more reliable intraday coverage, but they are
not required for multi-expiration rows.

## Verification

- `cargo test -p typhoon-engine --lib`: 1276 tests pass (+5:
  `third_friday_identification`, `triple_witching_months`,
  `classify_expiration_categories`, `market_calendar_emits_fridays`,
  `symbol_expirations_roundtrip`).
- `cargo build -p typhoon-native`: clean build.
- `docs/RESEARCH_PACKET.md`: new sub-block 2.264 (EXPCAL) added;
  envelope updated to account for the optional expirations block.

## Packet envelope delta

| Surface | Field count | Approx bytes when populated | Free / Paid |
|---|---|---|---|
| EXPCAL (Tier 2, up to 12 rows) | 6 header fields + 10 per row | ~400–1200 | Free (options chain cache) |

Envelope: 80–152 KB → 80–153 KB single-symbol (only adds when the
chain is cached and classified, which is opt-in per symbol).
