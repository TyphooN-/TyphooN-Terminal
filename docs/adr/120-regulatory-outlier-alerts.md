# ADR-120: Regulatory Outlier Alerts Beside Chart Symbols

Status: Accepted
Date: 2026-06-13

## Context

Some symbols carry regulatory status that is not obvious from price, news, fundamentals, or SEC filings. WOK is currently on the Nasdaq Reg SHO Threshold List. That matters enough to be visible at the point of decision: the chart header next to the symbol.

A Reg SHO threshold security is an outlier condition, not normal market metadata. Hiding it in a research window or requiring a manual web lookup is too easy to miss.

## Decision

TyphooN Terminal will maintain a cached symbol-level regulatory alert layer and render active alerts as red badges attached to the chart symbol header.

Initial alert source:

- NasdaqTrader Reg SHO Threshold List
- Public daily text file under `https://www.nasdaqtrader.com/dynamic/symdir/regsho/`
- No API key, no paid API, no account required

Initial UI label:

- `!! Reg SHO !!`

Storage:

- SQLite table `regulatory_alerts`
- keyed by `(symbol, kind, source)`
- stores label, source, as-of date, details, updated timestamp

Refresh behavior:

- background thread refreshes NasdaqTrader Reg SHO periodically
- cached alerts are read into `BgData`
- chart rendering consumes in-memory `regulatory_alerts_by_symbol`
- no per-frame network or database lookup

Symbol normalization:

- chart symbols such as `WOK.EQ` normalize to `WOK`
- Nasdaq-listed symbols are stored uppercase

## Why not require an API?

No API is needed for Reg SHO. NasdaqTrader publishes a daily machine-readable pipe-delimited text file. The app can use the public TXT feed directly and cache it locally.

An API may be needed later for other regulatory/status sources if they lack public downloadable files, but Reg SHO specifically does not require one.

## Consequences

Positive:

- Reg SHO and similar outlier conditions become visible exactly where the user looks before trading.
- Works offline after the latest successful refresh.
- Avoids adding another credential/API dependency.
- Keeps regulatory warning rendering O(1) in the chart path.

Negative / risks:

- NasdaqTrader availability can fail or be delayed; stale cached data may persist until the next successful refresh.
- This is an informational alert, not legal/compliance advice.
- Additional alert sources will need source-specific parsing and stale-data policy.

## Future Extensions

Possible additional sources:

- exchange halt / LULD / trading pause feeds
- short sale restriction lists
- exchange delisting / non-compliance notices
- hard-to-borrow or borrow-rate feeds if a reliable source is available
- SEC / FINRA outlier datasets when machine-readable and useful

Each source should feed the same `regulatory_alerts` table and render as compact chart-header badges.
