# ADR-126: Primary / Assist Broker Selection

**Status:** Accepted | **Date:** 2026-06-22

> **Scope note (2026-07):** the Primary/Assist choice governs **order routing**
> and the **rank-0 equity-merge lane** — it does **not** gate live market data.
> L1/L2/L3 is **capability- and symbol-routed** via the model in
> [ADR-129](129-l1-l2-l3-market-data-support.md) (`OrderBroker::market_data_capabilities`,
> `common.rs::depth_stream_supported`): depth is served by whichever enabled
> broker can serve the symbol (e.g. Kraken streams `BTC/USD` depth even while
> Alpaca is Primary). So switching Primary never disables L1/L2/L3 that another
> enabled broker still provides.

Builds on **ADR-009** (Multi-Broker Architecture), **ADR-111** (broker scope =
Kraken + Alpaca only), **ADR-112** (demand-depth vs catalog-breadth sync lanes),
and **ADR-113 / ADR-124** (cross-source equity merge & its symmetric
scale-protection). Touches `OrderBroker`
(`typhoon_engine::broker::protocol`), `resolve_order_broker`
(`app/trade_ops.rs`), the top-bar switch (`app/app_runtime_menu.rs`), and the
equity merge ranking (`app/chart/equity_merge.rs`).

## Context

Two problems, one decision:

1. **The order-routing target could not be changed.** `resolve_order_broker`
   ran every frame (and again at order submit) and **unconditionally** forced the
   routing target back to Kraken whenever Alpaca was in paper mode and Kraken was
   live (a "Live > Paper" bias). Picking **Alpaca** in the Trading-panel `Broker`
   combo snapped straight back to Kraken, and a paper-Alpaca selection would have
   been silently re-routed to live Kraken at submit. The bias was a hard lock, not
   a default.

2. **"Primary broker" was implicit and hardwired to Kraken.** Kraken was the de
   facto main broker — the order-routing default *and* the trusted/native source
   for the equity data merge (`kraken-equities` rank 0) — with Alpaca as the
   assist/gap-fill lane. There was no way to say "use Alpaca as the primary broker"
   and have Kraken become the assist. The product also needs to generalize to **N
   brokers** later (one primary, the rest assist), so the model must not hardcode a
   2-way Alpaca/Kraken split.

## Decision

Introduce an explicit, persisted **primary broker**, selectable from the top bar,
that governs both order routing and the equity-merge trust orientation. Every
other enabled broker is an **assist** lane.

### Broker identity & state

- `OrderBroker` (`Alpaca | Kraken`) becomes the **broker-identity** enum. New
  brokers are added as variants plus match arms; nothing else hardcodes the split.
  Helpers: `equity_source_tag()` (`"alpaca"` / `"kraken-equities"` — bridges the
  identity to the merge's string-keyed sources), `as_persist_str()` /
  `from_persist_str()`, and `enabled_cycle(alpaca_enabled, kraken_enabled)` (the
  ordered switch cycle, enabled brokers only).
- `primary_broker: OrderBroker` is added to app state, **default Kraken** (so an
  upgraded session reproduces today's behavior until the user flips it), persisted
  in the `app:sync_preferences` blob.

### Top-bar switch

A `Primary: <BROKER>` button beside the **Scope** indicator (same styling), shown
**only when 2+ brokers are enabled** — with a single broker there is nothing to
re-prioritize, so both Scope and Primary are hidden. Left-click cycles enabled
brokers, sets `primary_broker`, points `order_broker` at it (the per-trade combo
can still override afterward), mirrors the choice into the merge, and persists.

### Order routing (bug fix)

`resolve_order_broker` now **only** normalizes when the current target is
*unavailable* (broker disabled/disconnected); an explicit, available selection is
never overridden. On fallback it prefers `primary_broker`, then any other
available broker. The per-frame "paper Alpaca → live Kraken" force is deleted.

### Equity data-merge inversion (gated, reversible)

The merge's source priority — which source defines the trusted price scale vs.
which only corroborates/gap-fills — was a single function,
`chart_equity_source_rank` (`kraken-equities` = 0, `alpaca` = 2, `yahoo` = 3,
`default` = 4). It is now **primary-aware**:

- Pure core `chart_equity_source_rank_for(source, primary)`: the **primary**
  broker's equity source is rank 0 (defines the scale); the other tradeable broker
  is rank 2 (trusted-tier assist — corroborates/gap-fills but cannot redefine the
  scale); Yahoo/default stay ranks 3/4. Swapping which tradeable source is 0 vs 2
  is the entire inversion.
- The merge runs on many background cache/load threads that do not carry app
  state, so the single app-level choice is mirrored into a process-wide atomic
  (`MERGE_PRIMARY_BROKER`) that the no-arg `chart_equity_source_rank` and
  `chart_merge_equity_raw_bars` read. The app updates it on the switch and on
  session load. Tests pin orientation with the pure `*_for` /
  `chart_merge_equity_raw_bars_with_primary` variants (no global mutation).
- Low-TF (M1/M5) equity merges, which accept only the *native* source, now use the
  **primary** broker's source tag instead of a hardcoded `kraken-equities`.

This is safe because the merge's scale-protection is **symmetric** (ADR-124): the
"≥2 stable divergent eras" / `SCALE_CAP` guard protects whichever source is
trusted, in either orientation. With **Kraken primary** (the default) every rank
is identical to pre-ADR-126, so there is **zero regression**.

### Account primacy

`selected_trade_account_snapshots` orders the primary broker first, then assists,
so the primary account leads the Trading panel.

## Consequences

- The user can set **Alpaca as primary**: orders route to Alpaca by default and
  Alpaca defines the chart/analytics price scale, with Kraken as the assist
  gap-fill — and the choice persists. Flipping back to Kraken primary reproduces
  the prior behavior exactly.
- **Scope of "inversion":** this inverts the **trust/merge** orientation (which
  source defines the price series) and **order routing / account** primacy — the
  parts that decide what price you see and where orders go. The xStocks
  **reference catalog** / broad sync-target universe (ADR-112's catalog-breadth
  lane) remains Kraken-equities-anchored; both brokers' lanes keep running and the
  *merge* decides trust. Physically re-rooting the sync-target universe onto an
  Alpaca catalog is out of scope (it is a coverage concern, not a merge lane, and
  would disturb the Cloudflare-paced iapi budget for no charting benefit).
- **Merged cache self-heals.** `merged:SYM:TF` rows are rebuilt from provider rows
  on materialize/load (ADR-124), so rows cached under the previous primary's scale
  are overwritten on the next materialize for active/charted symbols — no purge.
- **N-broker ready.** Adding a broker = a new `OrderBroker` variant + its
  `equity_source_tag()`; the switch, routing, persistence, account ordering, and
  merge ranking all follow without further special-casing.
- **Tests:** `chart_equity_source_rank_inverts_with_primary_broker`,
  `chart_equity_merge_trusted_scale_follows_primary_broker`, and
  `order_broker_persistence_and_cycle_helpers` cover the inversion, merge
  precedence, and the identity/persistence helpers; the existing merge suite
  continues to pass at the Kraken-primary default.
