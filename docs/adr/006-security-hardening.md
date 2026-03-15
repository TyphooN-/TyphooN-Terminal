# ADR-006: Security Hardening

**Status:** Implemented (Pass 6)
**Date:** 2026-03-15
**Updated:** 2026-03-15

## Pass 1 — Initial Hardening

### Critical → Fixed

1. **CSP enabled**: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'` — prevents external script injection
2. **Devtools debug-only**: Moved to `[features] devtools = ["tauri/devtools"]` — not included in release builds

### High → Fixed

3. **XSS in autocomplete**: Replaced `innerHTML` with `createElement` + `textContent`
4. **XSS in log panel**: Replaced `innerHTML` with `createElement` + `textContent`
5. **XSS in tooltip**: Changed to `textContent` with `pre-line` whitespace
6. **Input validation on orders**: Symbol, qty, side validated in Rust

## Pass 2 — Comprehensive Audit & Hardening

Full security audit of all Rust and JavaScript source files.

### Critical → Fixed

7. **Header parsing panic**: `self.api_key.parse().unwrap()` in `alpaca.rs` → graceful fallback
8. **SEC EDGAR URL injection**: String concatenation → reqwest `.query()` parameterized
9. **Unbounded `fetch_article()`**: Now HTTPS-only, 10s timeout, 2MB max response, generic User-Agent

### High → Fixed

10. **Weak symbol validation**: Replaced with strict `is_valid_symbol()`: 1-10 chars, ASCII alphanumeric + `/` + `.`, max 1 slash
11. **Timeframe validation**: `is_valid_timeframe()` whitelist
12. **Path traversal in cache**: `canonicalize()` check + `.zst` extension guard
13. **Cache key injection**: Reject `..`, `/`, `\` in cache keys
14. **No HTTP timeouts**: 30s on Alpaca client, 10-15s on SEC EDGAR
15. **Input bounds**: `limit` capped at 50K (bars), 50 (news), 10 (timeframes)

### Medium → Fixed

16. **CSP strengthened**: Added `connect-src`, `frame-ancestors`, `base-uri`, `form-action`
17. **Unused shell plugin removed**: `tauri-plugin-shell` removed from Cargo.toml + main.rs
18. **Error message sanitization**: Generic messages for external requests
19. **User-Agent cleanup**: Removed personal email, generic UA for article fetch

## Pass 3 — Full Coverage Audit

Re-audited every Tauri command and frontend file after Pass 2 changes. Found and fixed remaining gaps:

### High → Fixed

20. **Missing validation on 8 commands**: `close_position`, `get_asset`, `get_corporate_actions`, `calculate_lots`, `calculate_position_var`, `set_sl_level`, `set_tp_level`, `get_sl_tp_pl`, `open_martingale_hedge` — all now validate symbol with `is_valid_symbol()`
21. **No finite/positive checks on financial inputs**: `sl_price`, `tp_price`, `current_price`, `position_size`, `entry_price` now validated with `is_finite()` + positive checks
22. **Side validation on `get_sl_tp_pl`**: Must be `"long"` or `"short"`
23. **API key format validation**: `connect()` now validates keys are non-empty, ≤100 chars, ASCII alphanumeric
24. **Qty validation on `close_position`**: Optional qty now checked for positive + finite
25. **News error leaks API response body**: `resp.text()` body no longer included in error message

### Medium → Fixed

26. **`search_symbols` query unbounded**: Capped at 50 chars
27. **All `innerHTML = ""` in frontend**: Changed to `textContent = ""` — eliminates last DOM injection surface
28. **Zero `unwrap()` on user input in alpaca.rs**: Confirmed none remain after Pass 2 fixes

### Accepted Risks (Documented)

29. **Credentials in localStorage**: Tauri webview localStorage is sandboxed per application. OS keychain is a future improvement. Users can uncheck "Save credentials"
30. **`withGlobalTauri: true`**: Required for Tauri invoke(). CSP prevents external scripts from exploiting this
31. **API keys in Rust memory**: Process memory access requires root/admin. `zeroize` crate is future work
32. **`unsafe-inline` in style-src**: Required by lightweight-charts for dynamic canvas styling
33. **`serde_json::to_string().unwrap()` on own structs**: These serialize our own `#[derive(Serialize)]` types — cannot fail

## Validation Checklist

- [x] `cargo check` — clean compile, no warnings
- [x] `npx vite build` — clean build
- [x] **Every** symbol-accepting command has `is_valid_symbol()` (17 call sites)
- [x] **Every** timeframe-accepting command has `is_valid_timeframe()`
- [x] **Every** financial value has `is_finite()` + bounds checks
- [x] No `.unwrap()` on user-provided input parsing (0 in alpaca.rs, 0 in main.rs input paths)
- [x] All HTTP clients have explicit timeouts (30s broker, 10-15s SEC, 10s article)
- [x] All external URL fetches validate scheme (HTTPS only)
- [x] All cache operations validate paths (canonicalize + extension guard)
- [x] No unused plugins in Tauri config
- [x] No `innerHTML` with any value other than clearing in frontend
- [x] API keys validated on connect (format + length)
- [x] News/SEC errors don't leak response bodies

## Pass 4 — Resource Limits & Dependency Cleanup

### Medium → Fixed

34. **Discord webhook — no timeout**: `Client::new()` replaced with 10s timeout builder
35. **Discord webhook — unbounded message**: Capped at 2000 chars (Discord's limit)
36. **`set_risk_config` — no bounds on deserialized values**: Now validates `risk_pct` (0-100), `max_risk_pct` (0-100), `var_confidence` (0-1). JSON input capped at 4KB
37. **`set_martingale_config` — no bounds on deserialized values**: Now validates margin thresholds non-negative, spread tolerance non-negative. JSON input capped at 4KB
38. **`save_cold_cache` — unbounded data**: Capped at 50MB uncompressed
39. **`load_cold_cache` — zstd bomb risk**: Compressed files capped at 10MB, decompressed output capped at 50MB
40. **4 unused dependencies**: Removed `tokio-tungstenite`, `futures-util`, `toml`, `url` — reduces supply chain attack surface and compile time

## Pass 5 — Logic Bugs & Correctness

### High → Fixed

41. **Operator precedence bug in break-even detection** (main.rs:299): `p.symbol == symbol || p.symbol == symbol_no_slash && { SL check }` — `&&` binds tighter than `||`, so the raw-symbol match skipped the SL proximity check entirely. Any position matching the symbol was treated as break-even, reducing risk allocation incorrectly. Fixed with explicit parentheses: `(p.symbol == symbol || p.symbol == symbol_no_slash) && { SL check }`
42. **Crypto symbol in URL path** (alpaca.rs:264,306): `close_position("BTC/USD")` → `positions/BTC/USD` caused Alpaca 404. `get_asset("BTC/USD")` same issue. Fixed: URL-encode slash as `%2F` for path segments

### Medium → Fixed

43. **Double-order guard**: Frontend "Open Trade" button now sets `orderInFlight` flag during execution — prevents double-click or keyboard spam (`t` key) from placing duplicate orders. Guard released in `finally` block even on error
44. **DOMParser on untrusted HTML**: `openArticleInline` parses fetched article HTML. Mitigated by: CSP blocks inline scripts/event handlers, `textContent` extraction only (no innerHTML), `<script>/<style>/<iframe>` tags stripped pre-extraction. **Accepted risk** — defense in depth adequate

## Validation Checklist (Updated)

- [x] `cargo check` — clean compile, zero warnings
- [x] `npx vite build` — clean build
- [x] **Every** symbol-accepting command has `is_valid_symbol()` (17 call sites)
- [x] **Every** timeframe-accepting command has `is_valid_timeframe()`
- [x] **Every** financial value has `is_finite()` + bounds checks
- [x] **Every** HTTP client has explicit timeout (10-30s)
- [x] **Every** config deserialization has JSON size cap + value bounds
- [x] **Every** cache operation has size limits (50MB write, 10MB compressed read, 50MB decompressed)
- [x] No `.unwrap()` on user-provided input parsing
- [x] All external URL fetches validate scheme (HTTPS only)
- [x] All cache operations validate paths (canonicalize + extension guard)
- [x] No unused plugins or dependencies
- [x] No `innerHTML` in frontend
- [x] API keys validated on connect (format + length)
- [x] News/SEC/Discord errors don't leak response bodies
- [x] Crypto symbols URL-encoded in path segments
- [x] Order placement guarded against double-fire
- [x] Break-even detection uses correct operator precedence
- [x] `set_risk_config` validates all 12 fields (bounds + timeframe whitelist)
- [x] Division-by-zero guard in MG sizing
- [x] Window event listeners cleaned up on close
- [x] `list_cold_cache` capped at 10K entries

## Pass 6 — Resource Leaks, Division Guards & Config Bounds

### Medium → Fixed

45. **`calc_open_mg_size` division by zero** (martingale.rs:276): `spread_tolerance = 0` → `equity / 0 = Infinity` → would attempt to place infinite lots. Added guard: returns `(0, 0)` if tolerance ≤ 0 or equity ≤ 0
46. **`set_risk_config` incomplete bounds**: Only 3 of 12 config fields were validated. Added bounds for: `fixed_lots` (0–1M), `fixed_orders` (≤100), `var_risk_pct` (0–100), `var_notional` (0–1B), `var_periods` (≤10K), `margin_buffer_pct` (0–100), `min_balance` (≥0), `additional_risk_ratio` (0–10), `var_timeframe` (whitelist)
47. **Window event listener leak** (windows.js): Each `createWindow()` added 4 `document`-level listeners (`mousemove`×2, `mouseup`×2) that were never removed on close. After opening/closing many windows, hundreds of stale handlers accumulated. Fixed: named handlers + `removeEventListener` in close callback
48. **`list_cold_cache` unbounded**: Could enumerate millions of cache files into one JSON response. Capped at 10,000 entries

### Accepted

49. **`fetch_article` SSRF via localhost HTTPS**: Low-risk in desktop app context — no multi-tenant concern, user controls the URL. Documented
50. **`windowZIndex` unbounded counter**: Cosmetic only — browser handles integer overflow gracefully

## Remaining Work (Future Passes)

- ~~`zeroize` crate for API key memory cleanup~~ ✅ Done (Pass 7)
- ~~Frontend rate limiting (debounce rapid-fire order clicks)~~ ✅ Done (Pass 7 — all 4 trading buttons)
- ~~`cargo audit` and `npm audit`~~ ✅ Clean (0 vulnerabilities, 18 allowed GTK warnings)
- OS keychain for credential storage (via `keyring` crate)
- Certificate pinning for Alpaca API endpoints (TLS 1.2+ min with rustls)
- Restrict Tauri command allowlist per window (N/A — single window app)
