use super::*;

// ─── application state ───────────────────────────────────────────────────────

/// Upcoming event source filter for the Event Calendar window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EventSource {
    All,
    Alpaca,
    Kraken,
    Positions,
}

/// Upcoming event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventKind {
    Earnings,
    ExDividend,
    DividendPayment,
}

impl EventKind {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Earnings => "Earnings",
            Self::ExDividend => "Ex-Div",
            Self::DividendPayment => "Div Pay",
        }
    }
}

/// Single upcoming event row — used by the Event Calendar window.
#[derive(Debug, Clone)]
pub(crate) struct EventRow {
    pub(crate) symbol: String, // bare ticker (e.g. AAPL)
    pub(crate) company: String,
    pub(crate) date: String,    // YYYY-MM-DD
    pub(crate) days_until: i64, // days from today (negative = past)
    pub(crate) kind: EventKind,
    pub(crate) detail: String, // yield%, previous EPS, etc.
    pub(crate) in_alpaca: bool,
    pub(crate) in_kraken: bool,
}

/// Background-computed data — populated by background thread, read by render thread.
/// This eliminates SQLite queries from the render loop.
#[derive(Default, Clone)]
pub(crate) struct BgData {
    pub(crate) cache_stats: Option<(i64, i64, i64)>,
    pub(crate) sec_filings: Vec<sec_filing::SecFiling>,
    pub(crate) sec_alerts: Vec<sec_filing::FilingAlert>,
    pub(crate) detailed_stats: Vec<(String, i64, i64)>,
    /// Per-key compressed blob size in bytes. Populated alongside `detailed_stats`
    /// from the same BG query so Storage Manager can show per-row KB/MB/GB
    /// without re-scanning the DB. Separate HashMap instead of widening
    /// `detailed_stats` to a 4-tuple so the ~25 existing consumers stay untouched.
    pub(crate) cache_blob_sizes: std::collections::HashMap<String, i64>,
    /// Cached first/last bar timestamps per key + the write_ts snapshot we
    /// extracted it under (so staleness can be detected without re-hashing
    /// the whole blob). Value tuple = (first_ms, last_ms, cached_for_write_ts).
    /// Populated incrementally by the BG thread from the TTBR blob headers;
    /// rate-limited to avoid blowing the 3 s cycle budget on cold startup
    /// when thousands of entries need decompression. Storage Manager and the
    /// crypto backfill window both read directly from this map.
    pub(crate) bar_ts_cache: std::collections::HashMap<String, (i64, i64, i64)>,
    /// Per-source `(symbol, timeframe) -> SyncCacheState` maps keyed by the
    /// `"<source>:"` cache-key prefix (e.g. `"alpaca:"`), built in ONE pass over
    /// `detailed_stats` on the BG worker. The sync scheduler reads these instead
    /// of rescanning the whole catalog per lane on the render thread (the
    /// recurring ~130ms `pre_broker` hitch). See `build_source_sync_state_maps`.
    pub(in crate::app) source_sync_state: std::collections::HashMap<
        &'static str,
        std::collections::HashMap<(String, String), SyncCacheState>,
    >,

    // ── SEC / Insider ──
    pub(crate) insider_trades: std::collections::HashMap<String, Vec<sec_filing::InsiderTrade>>,
    pub(crate) sec_content_stats: (usize, usize), // (total_filings, indexed_content)

    // ── Fundamentals (cached from background thread) ──
    pub(crate) all_fundamentals: Vec<fundamentals::Fundamentals>,
    pub(crate) upcoming_earnings: Vec<(String, String, String)>,
    pub(crate) upcoming_dividends: Vec<(String, String, String, Option<f64>)>,
    /// Active symbol-level regulatory warnings keyed by normalized ticker.
    /// Populated by the background thread from cached public outlier lists
    /// (currently NasdaqTrader Reg SHO threshold securities).
    pub(crate) regulatory_alerts_by_symbol:
        std::collections::HashMap<String, Vec<regulatory_alerts::RegulatoryAlert>>,
}

/// Bottom panel mode.
#[derive(PartialEq)]
pub(crate) enum BottomTab {
    Log,
}

/// FA window — which statement is currently shown.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum FinancialsView {
    Income,
    Balance,
    CashFlow,
}

/// FA window — annual vs quarterly reporting period.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum FinancialsPeriod {
    Annual,
    Quarterly,
}

/// RESEARCH_PACKET viewer tree node — one heading row in the
/// left-hand navigation of the packet viewer window. Depth maps to
/// markdown header level: 2 = `## `, 3 = `### `, 4 = `#### `.
#[derive(Clone, Debug, Default)]
pub(crate) struct PacketTreeNode {
    pub(crate) title: String,
    pub(crate) depth: u8,
    pub(crate) byte_offset: usize, // offset into packet_viewer_text where the header starts
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ImportedResearchArtifact {
    pub(crate) symbol: String,
    pub(crate) report_date: String,
    pub(crate) filename: String,
    pub(crate) source_path: String,
    pub(crate) imported_at: String,
    pub(crate) content: String,
}

/// Right panel section tabs (matching old WebKit layout).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum RightTab {
    Trading,
    Positions,
    Orders,
    Watchlist,
    Risk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RightPanelSectionId {
    Trading,
    Positions,
    RecentFills,
    Orders,
    Watchlist,
    Risk,
    News,
    MtfGrid,
}

impl RightPanelSectionId {
    pub(crate) const DEFAULT_ORDER: [Self; 8] = [
        Self::Trading,
        Self::Positions,
        Self::RecentFills,
        Self::Orders,
        Self::Watchlist,
        Self::Risk,
        Self::News,
        Self::MtfGrid,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Trading => "trading",
            Self::Positions => "positions",
            Self::RecentFills => "recent_fills",
            Self::Orders => "orders",
            Self::Watchlist => "watchlist",
            Self::Risk => "risk",
            Self::News => "news",
            Self::MtfGrid => "mtf_grid",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Trading => "Trading",
            Self::Positions => "Positions",
            Self::RecentFills => "Recent Fills",
            Self::Orders => "Orders",
            Self::Watchlist => "Watchlist",
            Self::Risk => "Risk & Account",
            Self::News => "News",
            Self::MtfGrid => "MTF Grid",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "trading" => Some(Self::Trading),
            "positions" => Some(Self::Positions),
            "recent_fills" => Some(Self::RecentFills),
            "orders" => Some(Self::Orders),
            "watchlist" => Some(Self::Watchlist),
            "risk" => Some(Self::Risk),
            "news" => Some(Self::News),
            "mtf_grid" => Some(Self::MtfGrid),
            _ => None,
        }
    }
}

pub(crate) const KRAKEN_TRADE_HISTORY_CAP: usize = 20_000;

/// Risk sizing mode (old app had dropdown).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum RiskMode {
    VaR,
    Standard,
    Fixed,
    Dynamic,
    KrakenPro,
}

impl RiskMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            RiskMode::VaR => "VaR",
            RiskMode::Standard => "Standard",
            RiskMode::Fixed => "Fixed",
            RiskMode::Dynamic => "Dynamic",
            RiskMode::KrakenPro => "KrakenPro",
        }
    }
}

/// Reusable sort state for clickable column headers.
#[derive(Clone, Default)]
pub(crate) struct SortState {
    pub(crate) column: usize,   // which column is sorted (0-indexed)
    pub(crate) ascending: bool, // true = ascending, false = descending
}

impl SortState {
    pub(crate) fn toggle(&mut self, col: usize) {
        if self.column == col {
            self.ascending = !self.ascending;
        } else {
            self.column = col;
            self.ascending = true;
        }
    }

    /// Render a clickable header label. Returns true if clicked.
    pub(crate) fn header(ui: &mut egui::Ui, label: &str, col: usize, state: &SortState) -> bool {
        let arrow = if state.column == col {
            if state.ascending {
                " \u{25B2}"
            } else {
                " \u{25BC}"
            }
        } else {
            ""
        };
        let text = format!("{}{}", label, arrow);
        let color = if state.column == col {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(120, 120, 140)
        };
        ui.add(
            egui::Label::new(egui::RichText::new(text).color(color).small().strong())
                .sense(egui::Sense::click()),
        )
        .clicked()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct BookmapWindowState {
    pub(crate) symbol: String,
    pub(crate) open: bool,
    /// Selected order_id for L3 Bookmap (demo/sim friendly; highlights row/header/marker and supports copy).
    /// Works for both sim and real L3 data. Real full actions gated by entitlements.
    pub(crate) selected_order_id: Option<String>,
}

/// One extra broker account slot (2–4). Credentials are stored in the keyring
/// (per-slot keys, written as soon as the Settings field is edited); only the
/// Paper/Live mode is persisted with the session (ADR-130). `paper` is
/// Alpaca-only; Kraken ignores it. Every configured slot joins the data-sync
/// rotation and is a valid trade/TradeCopy target — the per-slot
/// label/trade/data toggles were removed so all slots behave identically.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct ExtraAccountConfig {
    #[serde(skip)]
    pub(crate) api_key: String,
    #[serde(skip)]
    pub(crate) secret: String,
    pub(crate) paper: bool,
}

/// Alpaca retry-queue entry. Persisted as JSON under KV key `alpaca:retry_queue`
/// so 429'd symbol/TF pairs resume after app restart.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AlpacaRetry {
    pub(crate) symbol: String,
    pub(crate) timeframe: String,
    pub(crate) last_attempt: i64, // unix seconds of last dispatch
    pub(crate) next_attempt: i64, // unix seconds — earliest the retry worker may re-fire
    pub(crate) retry_count: u32,
    pub(crate) last_error: String,
    /// True if a prior fetch returned some bars but was cut short by 429 —
    /// tells the coverage sweep this is genuinely incomplete, not "no history."
    pub(crate) partial: bool,
}
