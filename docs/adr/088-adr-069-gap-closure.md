# ADR-088 — Close ADR-069 Feature Gap List

**Status:** Implemented
**Date:** 2026-04-09

## Context

ADR-069 (feature status 2026-04-05) documented a list of concrete gaps at that snapshot. Follow-up passes cleared most of them; this ADR closes the remaining actionable items from both the HIGH and LOW lists.

## Items from ADR-069 addressed by this ADR

### HIGH priority

**tastytrade position close** — "no mechanism exists"
Added `close_equity_position(symbol)` to `engine/src/broker/tastytrade.rs`. Looks up the current position, determines closing action (`Sell to Close` for long, `Buy to Close` for short) from the `quantity_direction` field, and submits a market order at full size via `place_equity_order`.

New `BrokerCmd::TastytradeClosePosition { symbol }` dispatches to the broker handler. In the right-panel tasty positions list, each row now has a small `X` close button next to the P/L with a tooltip explaining the action. Clicking sends the close command and logs "Tastytrade: closing {sym} at market".

**Periodic MT5 sync loop** — "currently manual command only"
New `mt5_auto_sync: bool` field, persisted across sessions. When enabled and at least one MT5 DB path is configured, fires `BrokerCmd::Mt5Sync` every ~5 minutes (1200 frames @ 4fps) silently without log spam.

Settings window exposes the toggle as a checkbox next to the manual "Sync MT5 Data Now" button with a tooltip explaining the behavior. Opt-in (default off) so existing user expectations are preserved.

**Options Greeks display in option chain windows** — already shipped in ADR-083 for the tastytrade window. The Alpaca `OPTIONS` command is a JSON-dump-only path by design (data goes through a generic results pane); Greeks belong with the tastytrade chain which already has them. Item closed as "implemented elsewhere".

### LOW priority (nice-to-have)

**Watchlist update/delete** — "only create/read exist"
Delete was already wired (right-click menu → Remove). Added three reorder actions to the same context menu:
- **Move Up** — swap with preceding entry
- **Move Down** — swap with next entry
- **Move to Top** — `remove(idx)` + `insert(0, item)`

Each operation touches only the adjacent state; no animation, no drag-and-drop complexity. Simple and predictable.

**Drawing control point drag-to-resize** — "handles render but aren't interactive"
The handles WERE already interactive for a subset: TrendLine, ExtendedLine, Rectangle, Ellipse, ArrowLine, InfoLine, Channel, Ruler, Pitchfork, SchiffPitchfork, FiboExtension, Triangle.

Expanded the match to cover:
- `FibChannel` (three-point pattern)
- **Vec-of-points drawings** via direct indexing into the points vector:
  - `Polyline`, `PathDraw`, `Brush` (freehand/continuous)
  - `ElliottWave`, `ElliottDouble`, `ElliottTriangle`, `ElliottTripleCombo` (Elliott variants)
  - `HeadShoulders`, `XabcdPattern`, `AbcCorrection`, `AbcdPattern` (harmonic patterns)
  - `TrianglePattern`, `ThreeDrives`, `CypherPattern`

Now when the user hovers over a control point on any Elliott wave or harmonic pattern, they can drag just that point instead of the whole drawing. The fallback `_ => {}` still catches truly whole-drawing-only types.

**Account-history-based compound interest projection** — "Compound interest calc is theoretical"
Added a "Use My Equity Curve" button next to "Calculate" in the Compound Interest Calculator. When the DARWIN portfolio equity curve has ≥30 days of data:
1. Computes CAGR from the first and last data points: `(end/start)^(1/years) - 1`
2. Pre-fills the `Principal` field with the current equity
3. Pre-fills the `Annual Return` field with the observed CAGR
4. Logs the calculation

The button is disabled when there's insufficient data (tooltip still visible). User can then tweak contribution amount and years to project forward based on their real historical return, not a hand-guessed `10%` default.

## Items from ADR-069 explicitly NOT addressed (deferred with reason)

- ~~**EasyLanguage compiler**~~ **Implemented in ADR-089/090.** Full transpiler backend with 216 tests.
- ~~**thinkScript compiler**~~ **Implemented in ADR-089/090.** Full transpiler backend.
- **Forex cross-rate matrix** — already implemented as `FOREX` command per the ADR-069 status table.
- **Dark pool volume (SqueezMetrics)** — requires paid data feed access; researched but no free source.
- **OCO order type** — Alpaca limitation (as noted in ADR-069). Not a terminal-side gap.
- **Stop-limit order type** — already implemented per the status table (was in the checked-off list).

## ADR-073 deferred items (WASM web client Phase 2)

Status update (2026-04-10):
- ~~Order entry from phone~~ **Implemented in ADR-089.** Trade tab with broker dropdown, symbol, side, type, qty. Two-step confirm. Close/cancel buttons.
- ~~Indicators on phone~~ **Implemented in ADR-092.** Server-computed indicators via GetIndicators WebCmd, rendered as polyline overlays.
- Drawing tools / MTF grid on phone — still deferred (complex UI, low priority for mobile)
- DARWIN analytics on phone — **Implemented in ADR-093.** GetDarwinWeb command + DarwinWebUpdate push.
- Push notifications — **Implemented in ADR-092.** BarUpdate/PositionUpdate/AccountUpdate WebMsg push replaces polling.

## Tests

854 tests pass *(updated 2026-04-10 — was 697 at time of writing)*.
- 511 engine
- 216 mql5-compiler
- 78 native
- 49 web-protocol

`cargo audit`: clean aside from the known `paste` warning via `image` crate (unrelated transitive dep).

## Consequences

**Positive:**
- Tasty users can close positions directly from the right panel instead of opening their broker's web UI.
- Users with MT5 running in the background can now have their cache stay fresh automatically.
- Elliott wave / harmonic pattern drawings are fully editable — a user can tune a labeled pattern without redrawing from scratch.
- The compound interest calculator now grounds itself in the user's actual observed returns instead of a placeholder default.

**Trade-offs:**
- MT5 auto-sync is opt-in (not default-on) to preserve existing user expectations and because a misconfigured MT5 path would otherwise trigger sync errors every 5 minutes. Users who want it enable it in Settings.
- Watchlist reorder is neighbor-swap, not drag-and-drop. Drag-and-drop in egui requires a significant state-tracking scaffold (drag_id, drop targets, hover states) that's not justified for a 10-20 row list.
- The vector-indexed control-point drag for Elliott/harmonic patterns assumes `cp_idx` matches the on-screen handle order. Hit-testing already uses the same iteration order so this holds. Any future handle reorder would need to keep the hit-test and drag indices in sync.

## Related

- ADR-069 — Original feature gap list (this ADR closes its actionable items)
- ADR-083 — Analytics expansion (delivered Options Greeks in tastytrade window)
- ADR-087 — Prior ADR follow-up closure pass (084/085/086 items)
