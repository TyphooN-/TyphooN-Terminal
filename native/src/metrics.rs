//! Prometheus metrics endpoint for TyphooN Terminal.
//!
//! Exposes application metrics on an HTTP `/metrics` endpoint (default port 9090)
//! for scraping by Prometheus, Grafana Agent, or compatible collectors.

use prometheus::{Encoder, Gauge, GaugeVec, Opts, Registry, TextEncoder};
use std::sync::Arc;

/// Snapshot of application state used to update metric gauges.
#[derive(Debug, Default)]
pub struct MetricsSnapshot {
    /// (account_name, equity) pairs.
    pub account_equity: Vec<(String, f64)>,
    /// (account_name, open_position_count) pairs.
    pub positions_open: Vec<(String, f64)>,
    /// (account_name, var_pct) pairs.
    pub var_current: Vec<(String, f64)>,
    /// (account_name, drawdown_pct) pairs.
    pub drawdown_current: Vec<(String, f64)>,
    /// SQLite cache file size in bytes.
    pub cache_size_bytes: f64,
    /// Number of symbols stored in cache.
    pub cache_symbols_total: f64,
    /// (symbol, timeframe, bar_count) triples.
    pub bars: Vec<(String, String, f64)>,
    /// Unix timestamp of last MT5 sync.
    pub sync_last_timestamp: f64,
    /// (broker_name, connected_bool_as_f64) pairs.
    pub broker_connected: Vec<(String, f64)>,
    /// Number of active price alerts.
    pub alerts_active: f64,
    /// App uptime in seconds.
    pub uptime_seconds: f64,
}

/// Holds all Prometheus gauge handles for efficient updates.
pub struct MetricsRegistry {
    pub registry: Registry,
    equity: GaugeVec,
    positions: GaugeVec,
    var: GaugeVec,
    drawdown: GaugeVec,
    cache_size: Gauge,
    cache_symbols: Gauge,
    bars: GaugeVec,
    sync_ts: Gauge,
    broker: GaugeVec,
    alerts: Gauge,
    uptime: Gauge,
    // Kraken WS OHLC channel backpressure metrics (real, non-debug)
    kraken_ws_bar_channel_capacity: Gauge,
    kraken_ws_bar_channel_queued: Gauge,
}

impl MetricsRegistry {
    /// Construct the metrics registry. Returns `Err` only if prometheus rejects
    /// one of our metric names — should never happen at runtime with our static
    /// names, but we surface the error properly per ADR-082 (no .unwrap()).
    pub fn new() -> Result<Self, String> {
        let registry = Registry::new();

        let equity = GaugeVec::new(
            Opts::new("typhoon_equity_total", "Total equity per DARWIN account"),
            &["account"],
        )
        .map_err(|e| format!("equity metric: {e}"))?;

        let positions = GaugeVec::new(
            Opts::new("typhoon_positions_open", "Count of open positions"),
            &["account"],
        )
        .map_err(|e| format!("positions metric: {e}"))?;

        let var = GaugeVec::new(
            Opts::new("typhoon_var_current", "Current VaR percentage"),
            &["account"],
        )
        .map_err(|e| format!("var metric: {e}"))?;

        let drawdown = GaugeVec::new(
            Opts::new("typhoon_drawdown_current", "Current drawdown percentage"),
            &["account"],
        )
        .map_err(|e| format!("drawdown metric: {e}"))?;

        let cache_size = Gauge::new(
            "typhoon_cache_size_bytes",
            "SQLite cache file size in bytes",
        )
        .map_err(|e| format!("cache_size metric: {e}"))?;
        let cache_symbols = Gauge::new("typhoon_cache_symbols_total", "Number of symbols in cache")
            .map_err(|e| format!("cache_symbols metric: {e}"))?;

        let bars = GaugeVec::new(
            Opts::new("typhoon_bars_total", "Bar count per symbol and timeframe"),
            &["symbol", "timeframe"],
        )
        .map_err(|e| format!("bars metric: {e}"))?;

        let sync_ts = Gauge::new(
            "typhoon_sync_last_timestamp",
            "Unix timestamp of last MT5 sync",
        )
        .map_err(|e| format!("sync_ts metric: {e}"))?;

        let broker = GaugeVec::new(
            Opts::new(
                "typhoon_broker_connected",
                "1 if broker connected, 0 if not",
            ),
            &["broker"],
        )
        .map_err(|e| format!("broker metric: {e}"))?;

        let alerts = Gauge::new("typhoon_alerts_active", "Number of active price alerts")
            .map_err(|e| format!("alerts metric: {e}"))?;
        let uptime = Gauge::new("typhoon_uptime_seconds", "Application uptime in seconds")
            .map_err(|e| format!("uptime metric: {e}"))?;

        // Kraken WS channel backpressure metrics
        let kraken_ws_bar_channel_capacity = Gauge::new(
            "typhoon_kraken_ws_bar_channel_capacity",
            "Maximum capacity of the Kraken WS bar channel",
        )
        .map_err(|e| format!("kraken_ws_bar_channel_capacity metric: {e}"))?;

        let kraken_ws_bar_channel_queued = Gauge::new(
            "typhoon_kraken_ws_bar_channel_queued",
            "Current number of bars queued in the Kraken WS channel",
        )
        .map_err(|e| format!("kraken_ws_bar_channel_queued metric: {e}"))?;

        let reg = |collector: Box<dyn prometheus::core::Collector>| {
            if let Err(e) = registry.register(collector) {
                tracing::warn!("Metric registration failed (may be duplicate): {}", e);
            }
        };
        reg(Box::new(equity.clone()));
        reg(Box::new(positions.clone()));
        reg(Box::new(var.clone()));
        reg(Box::new(drawdown.clone()));
        reg(Box::new(cache_size.clone()));
        reg(Box::new(cache_symbols.clone()));
        reg(Box::new(bars.clone()));
        reg(Box::new(sync_ts.clone()));
        reg(Box::new(broker.clone()));
        reg(Box::new(alerts.clone()));
        reg(Box::new(uptime.clone()));
        reg(Box::new(kraken_ws_bar_channel_capacity.clone()));
        reg(Box::new(kraken_ws_bar_channel_queued.clone()));

        Ok(Self {
            registry,
            equity,
            positions,
            var,
            drawdown,
            cache_size,
            cache_symbols,
            bars,
            sync_ts,
            broker,
            alerts,
            uptime,
            kraken_ws_bar_channel_capacity,
            kraken_ws_bar_channel_queued,
        })
    }

    /// Update all gauges from a snapshot of current app state.
    pub fn update(&self, snap: &MetricsSnapshot) {
        for (acct, val) in &snap.account_equity {
            self.equity.with_label_values(&[acct]).set(*val);
        }
        for (acct, val) in &snap.positions_open {
            self.positions.with_label_values(&[acct]).set(*val);
        }
        for (acct, val) in &snap.var_current {
            self.var.with_label_values(&[acct]).set(*val);
        }
        for (acct, val) in &snap.drawdown_current {
            self.drawdown.with_label_values(&[acct]).set(*val);
        }
        self.cache_size.set(snap.cache_size_bytes);
        self.cache_symbols.set(snap.cache_symbols_total);
        for (sym, tf, count) in &snap.bars {
            self.bars.with_label_values(&[sym, tf]).set(*count);
        }
        self.sync_ts.set(snap.sync_last_timestamp);
        for (name, val) in &snap.broker_connected {
            self.broker.with_label_values(&[name]).set(*val);
        }
        self.alerts.set(snap.alerts_active);
        self.uptime.set(snap.uptime_seconds);
    }

    /// Update Kraken WS bar channel backpressure metrics.
    /// Called from the WS writer to expose real channel saturation.
    pub fn set_kraken_ws_bar_channel_stats(&self, capacity: f64, queued: f64) {
        self.kraken_ws_bar_channel_capacity.set(capacity);
        self.kraken_ws_bar_channel_queued.set(queued);
    }
}

/// Start the Prometheus metrics HTTP server on the given port.
///
/// Spawns an axum server as a background tokio task. The server serves
/// `/metrics` in Prometheus text exposition format.
pub fn start_metrics_server(
    rt: &tokio::runtime::Handle,
    registry: Arc<MetricsRegistry>,
    port: u16,
) {
    let reg = registry.clone();
    rt.spawn(async move {
        let app = axum::Router::new().route(
            "/metrics",
            axum::routing::get(move || {
                let reg = reg.clone();
                async move {
                    let encoder = TextEncoder::new();
                    let metric_families = reg.registry.gather();
                    let mut buffer = Vec::new();
                    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
                        tracing::warn!("Metrics encode failed: {e}");
                    }
                    let content_type = encoder.format_type().to_string();
                    (
                        [(axum::http::header::CONTENT_TYPE, content_type)],
                        String::from_utf8(buffer).unwrap_or_default(),
                    )
                }
            }),
        );

        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!(
            "Prometheus metrics server listening on http://{}/metrics",
            addr
        );
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!("Failed to bind metrics server on port {}: {}", port, e);
                return;
            }
        };
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Metrics server error: {}", e);
        }
    });
}
