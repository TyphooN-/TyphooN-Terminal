//! Broker-modular market-data capability model (ADR-129, ADR-009).
//!
//! Every surface that decides whether to offer L1/L2/L3 for a symbol — DOM,
//! Bookmap, watchlist, chart overlays, the depth-stream toolbar — should consult
//! this model instead of hard-coding a single broker. Adding a broker to
//! [`OrderBroker`] forces new arms in the exhaustive matches below, so no
//! L1/L2/L3 decision can silently keep a stale 2-broker assumption.
//!
//! Current providers: Alpaca + Kraken. Planned re-additions are documented
//! inline at the bottom of this file: tastytrade (equities L1/L2 via DXLink)
//! after the Alpaca/Kraken combover, then Binance (crypto L1/L2/L3) as a
//! plausible later venue. Both enter through this same model rather than
//! special-casing UI behavior.

use crate::broker::protocol::OrderBroker;

/// How a broker delivers a given market-data level for a *supported* symbol.
/// Variants are ordered weakest → strongest so `>=`/`max` are meaningful when
/// merging capabilities across brokers.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum MarketDataSupport {
    /// Level not offered by this broker at all.
    Unsupported,
    /// Available but delayed (e.g. 15-min) — usable for context, not execution.
    Delayed,
    /// On-demand point-in-time snapshot (REST). No live deltas.
    Snapshot,
    /// Live streaming updates (WebSocket deltas/ticks).
    Stream,
}

impl MarketDataSupport {
    /// True when any data (delayed/snapshot/stream) is obtainable.
    pub fn is_available(self) -> bool {
        !matches!(self, MarketDataSupport::Unsupported)
    }

    /// True only for live streaming — gates "Start Stream" affordances.
    pub fn is_live(self) -> bool {
        matches!(self, MarketDataSupport::Stream)
    }

    /// True for real-time-usable data (snapshot or stream, not delayed/absent)
    /// — gates on-demand "Fetch Snapshot" affordances.
    pub fn is_realtime(self) -> bool {
        matches!(
            self,
            MarketDataSupport::Snapshot | MarketDataSupport::Stream
        )
    }

    /// Short lowercase tag for status badges / provenance strings.
    pub fn label(self) -> &'static str {
        match self {
            MarketDataSupport::Unsupported => "unsupported",
            MarketDataSupport::Delayed => "delayed",
            MarketDataSupport::Snapshot => "snapshot",
            MarketDataSupport::Stream => "stream",
        }
    }
}

/// Which asset classes a depth level (L2/L3) covers for a broker. Keeps the UI
/// from offering depth on symbols a broker cannot serve (e.g. Alpaca L2 is
/// crypto-only; Kraken depth is spot + tokenized-equity/xStock pairs).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DepthAssetScope {
    /// No depth for any asset class.
    None,
    /// Crypto pairs only.
    CryptoOnly,
    /// Kraken spot + tokenized-equity (xStock) pairs.
    SpotAndXStock,
    /// Every tradable symbol the broker lists (e.g. a full equities L2 venue).
    All,
}

impl DepthAssetScope {
    pub fn covers_anything(self) -> bool {
        !matches!(self, DepthAssetScope::None)
    }
}

/// Normalized per-broker L1/L2/L3 capabilities. Built once per broker via
/// [`OrderBroker::market_data_capabilities`]; UI reads fields rather than
/// re-matching on broker identity at each call site.
#[derive(Clone, Copy, Debug)]
pub struct BrokerMarketDataCapabilities {
    pub broker: OrderBroker,
    pub l1: MarketDataSupport,
    pub l2: MarketDataSupport,
    pub l3: MarketDataSupport,
    /// Asset classes the L2 book covers.
    pub l2_scope: DepthAssetScope,
    /// Asset classes the L3 book covers.
    pub l3_scope: DepthAssetScope,
    /// L3 requires auth/entitlement before a real start (a sim/demo path may
    /// still run without it).
    pub l3_entitlement_gated: bool,
    /// One-line human summary for status badges / tooltips / help.
    pub notes: &'static str,
}

impl OrderBroker {
    /// L1 (best bid/ask + last + sizes + basic stats) support.
    pub fn l1_support(self) -> MarketDataSupport {
        match self {
            // Alpaca Market-Data WS (SIP if entitled, else free IEX): live quotes+trades.
            OrderBroker::Alpaca => MarketDataSupport::Stream,
            // Kraken ticker v2: live.
            OrderBroker::Kraken => MarketDataSupport::Stream,
        }
    }

    /// L2 (aggregated depth book) support.
    pub fn l2_support(self) -> MarketDataSupport {
        match self {
            // Alpaca: crypto orderbook REST snapshots only; no streaming/equity L2.
            OrderBroker::Alpaca => MarketDataSupport::Snapshot,
            // Kraken: book v2 with CRC32 deltas — live stream (spot + xStock).
            OrderBroker::Kraken => MarketDataSupport::Stream,
        }
    }

    /// L3 (order-by-order book) support.
    pub fn l3_support(self) -> MarketDataSupport {
        match self {
            // Alpaca: no L3 feed.
            OrderBroker::Alpaca => MarketDataSupport::Unsupported,
            // Kraken: level3 v2 (auth/entitlement-gated; sim/demo otherwise).
            OrderBroker::Kraken => MarketDataSupport::Stream,
        }
    }

    /// Full normalized capability descriptor for this broker.
    pub fn market_data_capabilities(self) -> BrokerMarketDataCapabilities {
        match self {
            OrderBroker::Alpaca => BrokerMarketDataCapabilities {
                broker: self,
                l1: MarketDataSupport::Stream,
                l2: MarketDataSupport::Snapshot,
                l3: MarketDataSupport::Unsupported,
                l2_scope: DepthAssetScope::CryptoOnly,
                l3_scope: DepthAssetScope::None,
                l3_entitlement_gated: false,
                notes: "L1 stream (SIP/IEX); L2 crypto REST snapshots only; no L3.",
            },
            OrderBroker::Kraken => BrokerMarketDataCapabilities {
                broker: self,
                l1: MarketDataSupport::Stream,
                l2: MarketDataSupport::Stream,
                l3: MarketDataSupport::Stream,
                l2_scope: DepthAssetScope::SpotAndXStock,
                l3_scope: DepthAssetScope::SpotAndXStock,
                l3_entitlement_gated: true,
                notes: "L1/L2 stream (book v2 CRC32); L3 stream auth/entitlement-gated (sim demo otherwise).",
            },
        }
    }
}

// ── Planned broker re-additions (documented, not yet enabled) ────────────────
// When these return they gain an `OrderBroker` arm; the exhaustive matches above
// then force their L1/L2/L3 capabilities + depth scopes to be declared here, in
// one place, before any UI can compile against the new broker:
//   • tastytrade — restore after the Alpaca/Kraken combover. Expected: L1 stream
//     (DXLink), L2 stream for equities/futures where entitled, no public L3.
//   • Binance — plausible later crypto venue. Expected: L1 stream, L2 stream
//     (diff-depth), L3-like trade stream; entitlement-free public market data.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_ordering_and_predicates() {
        assert!(MarketDataSupport::Stream > MarketDataSupport::Snapshot);
        assert!(MarketDataSupport::Snapshot > MarketDataSupport::Delayed);
        assert!(MarketDataSupport::Delayed > MarketDataSupport::Unsupported);

        assert!(MarketDataSupport::Stream.is_live());
        assert!(!MarketDataSupport::Snapshot.is_live());

        assert!(MarketDataSupport::Snapshot.is_realtime());
        assert!(MarketDataSupport::Stream.is_realtime());
        assert!(!MarketDataSupport::Delayed.is_realtime());

        assert!(MarketDataSupport::Delayed.is_available());
        assert!(!MarketDataSupport::Unsupported.is_available());
    }

    #[test]
    fn kraken_streams_all_three_levels() {
        let k = OrderBroker::Kraken;
        assert!(k.l1_support().is_live());
        assert!(k.l2_support().is_live());
        assert!(k.l3_support().is_live());
        let caps = k.market_data_capabilities();
        assert!(caps.l3_entitlement_gated);
        assert_eq!(caps.l2_scope, DepthAssetScope::SpotAndXStock);
    }

    #[test]
    fn alpaca_l2_is_snapshot_only_no_l3() {
        let a = OrderBroker::Alpaca;
        assert!(a.l1_support().is_live());
        // L2 exists but is not a live stream (crypto REST snapshots).
        assert!(a.l2_support().is_realtime());
        assert!(!a.l2_support().is_live());
        assert!(!a.l3_support().is_available());
        let caps = a.market_data_capabilities();
        assert_eq!(caps.l2_scope, DepthAssetScope::CryptoOnly);
        assert_eq!(caps.l3_scope, DepthAssetScope::None);
    }

    #[test]
    fn every_enabled_broker_declares_capabilities() {
        // Exhaustive over the same list the top-bar Primary switch cycles.
        for b in OrderBroker::enabled_cycle(true, true) {
            let caps = b.market_data_capabilities();
            assert_eq!(caps.broker, b);
            // L1 is the mandatory primary overlay for any broker we enable.
            assert!(caps.l1.is_available(), "{} must provide L1", b.label());
            // If a broker advertises L2/L3 stream, it must scope which assets.
            if caps.l2.is_live() {
                assert!(caps.l2_scope.covers_anything());
            }
            if caps.l3.is_available() {
                assert!(caps.l3_scope.covers_anything());
            }
        }
    }
}
