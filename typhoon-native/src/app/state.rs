//! Extracted from app.rs: state helpers.

mod broker_messages;
mod models;
mod watchlist;

pub(crate) use watchlist::{KrakenEquityQuoteMeta, WatchlistRow, watchlist_row_from_raw_bars};

use super::*;

pub(crate) use broker_messages::{
    BrokerCmd, BrokerMsg, OrderBroker, QuickTradePlan, TradeAccountSnapshot,
};
#[cfg(test)]
pub(crate) use broker_messages::{
    format_news_scope_scrape_start, is_fundamentals_provider_coverage_gap,
    normalize_fundamentals_scrape_symbol, should_emit_fundamentals_scrape_progress,
};
pub(crate) use models::{
    AlpacaRetry, BgData, BookmapWindowState, BottomTab, EventKind, EventRow, EventSource,
    FinancialsPeriod, FinancialsView, ImportedResearchArtifact, KRAKEN_TRADE_HISTORY_CAP,
    PacketTreeNode, RightPanelSectionId, RightTab, RiskMode, SortState,
};
#[cfg(test)]
pub(crate) use typhoon_engine::core::watchlist::{
    watchlist_cache_fallback_sources, yahoo_extended_quote_time_is_fresh,
    yahoo_market_state_allows_extended_quote,
};

pub struct TyphooNApp {
    /// Shared cache handle — opened once at startup.
    pub(crate) cache: Option<Arc<SqliteCache>>,
    /// Receiver for async cache open (delivered on first frame).
    pub(crate) cache_rx: Option<std::sync::mpsc::Receiver<Arc<SqliteCache>>>,
    /// Whether initial chart load has been done after cache arrived.
    pub(crate) cache_loaded: bool,
    /// Cache open error (shown in log if set).
    pub(crate) cache_err: Option<String>,

    /// Symbol input text in the toolbar.
    pub(crate) symbol_input: String,

    /// Primary chart (or charts[0] in grid mode).
    pub(crate) charts: Vec<ChartState>,
    /// MTF grid: how many columns to show.
    pub(crate) mtf_cols: usize,
    /// MTF grid enabled flag.
    pub(crate) mtf_enabled: bool,
    /// Base zstd level for foreground bar-cache writes. Auto-compact can promote rows to zstd-22 later.
    pub(crate) bar_zstd_level: i32,
    /// Auto-compact (zstd-22) scheduler — opt-out toggle. ADR-089.
    pub(crate) auto_compact_enabled: bool,
    /// User-configured cadence/window/row threshold for the auto-compact gate.
    pub(crate) auto_compact_schedule: auto_compact::Schedule,
    /// UTC ms timestamp of the last successful automated or manual compact run.
    /// Drives the cadence gate so weekly runs don't pile up.
    pub(crate) auto_compact_last_run_ms: i64,
    /// Set when a compact pass is in flight (manual or auto). Cleared by the
    /// "Compact complete:" handler in the OrderResult dispatch.
    pub(crate) auto_compact_in_progress: bool,
    /// UTC ms timestamp when the current compact run was dispatched.
    pub(crate) auto_compact_started_ms: i64,
    /// Latest skip reason (for Storage Manager readout).
    pub(crate) auto_compact_last_skip: Option<String>,
    /// Last frame-time we saw user input — used by the auto-compact idle gate.
    pub(crate) auto_compact_last_input_at: std::time::Instant,
    /// Throttle for the gate evaluation: only re-evaluate every ~minute so we
    /// don't burn CPU running the gate every frame.
    pub(crate) auto_compact_next_check_at: std::time::Instant,
    /// Canonical base TFs allowed for automated scrape/sync flows.
    /// Stored as cache suffixes (`1Min` .. `1Month`) so all broker backfill
    /// paths read the same config.
    pub(crate) enabled_sync_timeframes: std::collections::BTreeSet<String>,
    /// Optional startup hint for Alpaca historical data RPM. `0` means auto:
    /// start at Basic cadence, then upgrade when Alpaca rate-limit headers
    /// reveal a higher-tier plan.
    pub(crate) alpaca_historical_rpm_hint: u32,
    /// Runtime-only RPM observed from Alpaca `X-RateLimit-Limit` headers.
    /// Not persisted; a fresh session re-learns it unless the user pins a hint.
    pub(crate) alpaca_historical_rpm_observed: u32,
    /// Which chart cell is focused in MTF grid (click to select).
    pub(crate) mtf_focused: Option<usize>,
    /// Which tabs are visible in MTF grid (true = shown, per chart index).
    pub(crate) mtf_visible: Vec<bool>,
    /// In-flight guard for the MTF Grid background fill (`compute_mtf_grid_status`).
    /// The worker writes the unified result cache directly and sends `()` on
    /// completion; `Some` means a pass is running, so the render path never spawns a
    /// second one or back-fills the same cells twice.
    pub(crate) mtf_grid_rx: Option<std::sync::mpsc::Receiver<()>>,
    /// Active symbol at the last fill — a change retriggers the fill so the focused
    /// symbol's cells refresh immediately.
    pub(crate) mtf_grid_status_symbol: String,
    /// Signature of the open-chart (symbol, timeframe) set at the last fill. Opening
    /// or closing a chart changes it and retriggers the fill so a just-closed
    /// timeframe's cell repopulates from the cache. `0` = never computed.
    pub(crate) mtf_grid_status_open_sig: u64,
    /// When the fill last ran. Drives a self-terminating throttled refresh that keeps
    /// back-filling navbar cells until the cache is warm, then idles.
    pub(crate) mtf_grid_status_at: Option<std::time::Instant>,
    /// Deferred chart loads: indices of charts to load, one per frame (avoids startup freeze).
    pub(crate) deferred_chart_loads: VecDeque<usize>,
    /// Side index for O(1) duplicate suppression in `deferred_chart_loads`.
    pub(crate) deferred_chart_load_set: HashSet<usize>,
    /// Last time a deferred chart was synchronously loaded. Used to pace expensive
    /// cache reads + indicator/MTF recomputes so restored MTF grids don't monopolize
    /// consecutive UI frames during broad market-data sync.
    pub(crate) deferred_chart_last_load_at: std::time::Instant,
    /// Last empty deferred load attempt per chart source key. Prevents MTF render
    /// loops from re-queueing an empty provider/cache row every frame while still
    /// allowing broker fetch results to explicitly queue an immediate reload.
    pub(crate) deferred_chart_empty_load_at: std::collections::HashMap<String, std::time::Instant>,

    /// Command palette open state.
    pub(crate) command_open: bool,
    /// Raw user input in the command palette.
    pub(crate) command_input: String,
    /// Currently highlighted command in console (arrow key navigation).
    pub(crate) console_selected: usize,
    /// ADR-092: Recent commands (MRU, up to 10, shown when palette filter is empty).
    pub(crate) recent_commands: VecDeque<String>,
    /// ADR-092: Compact mode — hides indicators and sub-panes for minimal execution view.
    pub(crate) compact_mode: bool,
    /// Session persistence is armed after we've attempted an initial restore.
    pub(crate) session_state_ready: bool,
    /// Last persisted session snapshot used for debounced incremental saves.
    pub(crate) session_last_saved_json: String,
    /// Timestamp when the current unsaved session diff was first observed.
    pub(crate) session_dirty_since: Option<std::time::Instant>,
    /// Last time we scanned session state for incremental persistence.
    pub(crate) session_last_scan_at: std::time::Instant,
    /// Consecutive incremental-save scans that found no change. Drives an
    /// adaptive scan backoff (500ms→2s) so an idle terminal stops rebuilding and
    /// diffing the session JSON twice a second; reset to 0 on any detected change.
    pub(crate) session_idle_scans: u32,
    /// Monotonic write sequence for session persistence. Bumped on the UI thread
    /// each time a snapshot is issued to disk; paired with `session_write_gate`
    /// so a late background autosave can never clobber a newer synchronous save.
    pub(crate) session_save_seq: u64,
    /// Highest session-write sequence already persisted to disk. Shared with the
    /// off-thread autosave worker to keep disk writes strictly newest-wins.
    pub(crate) session_write_gate: Arc<std::sync::Mutex<u64>>,
    /// True while an off-thread session autosave is running; coalesces the
    /// per-frame incremental save so redundant writers don't pile up.
    pub(crate) session_save_in_flight: Arc<std::sync::atomic::AtomicBool>,

    // ── indicator overlay toggles ────────────────────────────────────────
    pub(crate) show_sma200: bool,
    pub(crate) show_sma100: bool,
    pub(crate) show_kama: bool,
    pub(crate) show_ema21: bool,
    pub(crate) show_bollinger: bool,
    pub(crate) show_rsi: bool,
    pub(crate) show_fisher: bool,
    pub(crate) show_macd: bool,
    pub(crate) show_volume_pane: bool,
    pub(crate) show_stochastic: bool,
    pub(crate) show_adx: bool,
    pub(crate) show_ichimoku: bool,
    pub(crate) show_wma: bool,
    pub(crate) show_hma: bool,
    pub(crate) show_psar: bool,
    pub(crate) show_atr_proj: bool,
    pub(crate) show_prev_levels: bool,
    pub(crate) show_pivots: bool,
    pub(crate) show_fractals: bool,
    pub(crate) show_harmonics: bool,
    pub(crate) show_auto_fib: bool,
    pub(crate) show_supply_demand: bool,
    pub(crate) show_ehlers_ss: bool,
    pub(crate) show_ehlers_decycler: bool,
    pub(crate) show_ehlers_itl: bool,
    pub(crate) show_ehlers_mama: bool,
    pub(crate) show_ehlers_ebsw: bool,
    pub(crate) show_ehlers_cyber: bool,
    pub(crate) show_ehlers_cg: bool,
    pub(crate) show_ehlers_roof: bool,
    pub(crate) show_cci: bool,
    pub(crate) show_williams_r: bool,
    pub(crate) show_obv: bool,
    pub(crate) show_momentum: bool,
    pub(crate) show_cmo: bool,
    pub(crate) show_qstick: bool,
    pub(crate) show_disparity: bool,
    pub(crate) show_bop: bool,
    pub(crate) show_stddev: bool,
    pub(crate) show_mfi: bool,
    pub(crate) show_trix: bool,
    pub(crate) show_ppo: bool,
    pub(crate) show_ultosc: bool,
    pub(crate) show_stochrsi: bool,
    pub(crate) show_var_oscillator: bool,
    pub(crate) show_better_volume: bool,
    pub(crate) show_sessions: bool,
    pub(crate) show_vol_heatmap: bool,
    pub(crate) show_vwap: bool,
    pub(crate) show_price_histogram: bool,
    pub(crate) show_supertrend: bool,
    pub(crate) show_donchian: bool,
    pub(crate) show_keltner: bool,
    pub(crate) show_regression: bool,
    pub(crate) show_squeeze: bool,
    pub(crate) show_fvg: bool,
    pub(crate) show_order_blocks: bool,

    /// Saved chart templates: name → indicator JSON snapshot.
    pub(crate) chart_templates: std::collections::HashMap<String, serde_json::Value>,

    /// Drawing interaction mode.
    pub(crate) draw_mode: DrawMode,
    /// Current drawing style (applied to new drawings).
    pub(crate) draw_width: f32,
    pub(crate) draw_line_style: LineStyle,
    /// Pre-placement color for new drawings (TradingView: choose before placing).
    pub(crate) draw_color: egui::Color32,
    /// OHLC snap (magnet) toggle.
    pub(crate) snap_enabled: bool,
    /// Sync drawings across all charts with the same symbol (cross-timeframe).
    pub(crate) drawings_cross_tf: bool,
    /// Cross-timeframe drawings: sync drawings across charts with same symbol.
    pub(crate) cross_tf_drawings: bool,
    /// Auto-scroll to latest bar when new data arrives. Toggle with FOLLOW command.
    pub(crate) follow_latest: bool,
    /// In-progress polyline points (used during PlacingPolyline mode).
    pub(crate) polyline_points: Vec<(usize, f64)>,
    /// In-progress Elliott Wave / ABC / H&S / XABCD multi-click points.
    pub(crate) multi_click_points: Vec<(usize, f64)>,
    /// In-progress brush/freehand points (accumulated while mouse held down).
    pub(crate) brush_points: Vec<(usize, f64)>,

    /// ADR-038 Phase 2: Pluggable data source manager.
    pub(crate) data_sources: typhoon_engine::core::data_source::DataSourceManager,

    /// Broker connection fields (Alpaca).
    pub(crate) broker_api_key: String,
    pub(crate) broker_secret: String,
    pub(crate) broker_paper: bool,
    /// Full bar-sync controls are deliberately separate from broker login.
    /// Off = light mode: account/trading plus targeted fetches for open charts,
    /// owned positions, open-order symbols, and the user's watchlist.
    pub(crate) alpaca_full_bar_sync_enabled: bool,
    pub(crate) alpaca_enabled: bool,

    /// Broker connection fields (Kraken).
    pub(crate) kraken_full_bar_sync_enabled: bool,
    pub(crate) kraken_api_key: String,
    pub(crate) kraken_api_secret: String,
    pub(crate) kraken_ws_api_key: String,
    pub(crate) kraken_ws_api_secret: String,
    pub(crate) kraken_enabled: bool,
    pub(crate) kraken_connected: bool,
    pub(crate) kraken_pairs_requested: bool,
    pub(crate) kraken_futures_requested: bool,
    pub(crate) kraken_balances: Vec<(String, f64)>,
    pub(crate) show_kraken_spot_sell_dialog: bool,
    pub(crate) kraken_spot_sell_asset: String,
    pub(crate) kraken_spot_sell_pair: String,
    pub(crate) kraken_spot_sell_available: f64,
    pub(crate) kraken_spot_sell_qty: f64,
    pub(crate) kraken_spot_sell_pct: f32,
    pub(crate) kraken_spot_buy_pct: f32,
    pub(crate) kraken_spot_buy_qty: f64,
    pub(crate) kraken_trades: VecDeque<typhoon_engine::broker::kraken::KrakenTrade>,
    pub(crate) kraken_trade_keys: std::collections::HashSet<String>,
    pub(crate) kraken_cost_basis: std::collections::HashMap<String, KrakenCostBasis>,
    pub(crate) kraken_open_orders: Vec<typhoon_engine::broker::kraken::KrakenOrder>,
    /// (symbol, tf_label) → epoch-ms anchor for "WS pushed a bar this recent
    /// for this key". The REST sync scheduler consults this to skip refetch
    /// while the WS feed is keeping the cache current. O(1) insert and
    /// lookup; entries are not actively pruned because the per-key check
    /// already age-bounds them with the TF period.
    pub(crate) kraken_ws_fresh_until: std::collections::HashMap<(String, String), i64>,
    /// Last snapshot-sweep attempt per `(symbol, tf)`. Distinct from
    /// `kraken_ws_fresh_until` (which is only set on a NON-empty bar commit):
    /// this records that we *tried*, so a pair Kraken serves no bars for backs
    /// off instead of being re-swept every cadence (which would wedge the
    /// high-timeframe-first sweep). Not REST-visible, so it can't suppress REST
    /// refetch the way a real freshness anchor would.
    pub(crate) kraken_ws_snapshot_attempt: std::collections::HashMap<(String, String), i64>,
    pub(crate) kraken_pairs: Vec<(String, String)>,
    /// Normalized pair/display symbols cached as a set so
    /// `kraken_spot_symbol_in_loaded_pairs` is O(1) — the previous linear
    /// `kraken_pairs.iter().any(...)` ran `normalize_pair_symbol` (allocating)
    /// twice per element on every sync-symbol audit, multiplying with O(n)
    /// `kraken_spot_symbol_scrape_enabled` callers.
    pub(crate) kraken_pairs_normalized: std::collections::HashSet<String>,
    pub(crate) kraken_futures_symbols: Vec<String>,
    /// Kraken public market-data scrape universe switches. These gate automated
    /// public OHLC/Futures scheduling so the scrape budget stays on instruments
    /// the user can actually trade.
    pub(crate) kraken_scrape_xstocks: bool,
    pub(crate) kraken_scrape_usd_crypto: bool,
    pub(crate) kraken_scrape_fiat_crypto: bool,
    pub(crate) kraken_scrape_crypto_crosses: bool,
    pub(crate) kraken_scrape_futures: bool,
    /// Backfill provider switches. These are source-specific fallbacks, not
    /// broker universe toggles: they supplement native broker bars without
    /// changing broker/account connectivity.
    pub(crate) backfill_alpaca_kraken_equities_enabled: bool,
    pub(crate) backfill_yahoo_chart_enabled: bool,
    /// Stream Kraken bar updates via WS v2 in addition to the REST scheduler.
    /// Subscribes to every spot pair across 1Min/5Min/15Min/30Min/1Hour/4Hour/
    /// 1Day/1Week so low-timeframe bars stay current without burning REST
    /// budget. On by default whenever Kraken is enabled — the OHLC channel
    /// is on Kraken's public WS endpoint (no auth needed) and is strictly
    /// better than REST alone for the low timeframes that REST can't keep
    /// up with. Toggle persists across sessions; flip it off only if you
    /// need to suppress the 8 TCP connections for testing or footprint.
    pub(crate) kraken_ws_ohlc_enabled: bool,
    /// Set once the WS OHLC pipeline has been kicked off this session so we
    /// don't re-spawn streamers if the broker thread emits more lifecycle
    /// events. Resets to false when the user toggles the setting off.
    pub(crate) kraken_ws_ohlc_started: bool,
    /// Exact WS v2 subscribe symbols already handed to the streamer pipeline.
    /// Lets Spot start from AssetPairs and xStocks join later from the
    /// instrument universe without duplicate subscriptions.
    pub(crate) kraken_ws_ohlc_streamed_pairs: std::collections::HashSet<String>,
    pub(crate) kraken_ws_ohlc_snapshot_sweep_last_schedule: std::time::Instant,
    pub(crate) kraken_ws_ohlc_snapshot_sweep_in_flight: bool,
    /// Suppress new snapshot-sweep dispatches until this instant after a connect
    /// failure (esp. WS-connect 429). Escalates per consecutive failure so the
    /// sweep stops refiring every cadence slot and self-feeding the Kraken
    /// WS-connect rate limiter.
    pub(crate) kraken_ws_ohlc_snapshot_sweep_backoff_until: Option<std::time::Instant>,
    pub(crate) kraken_ws_ohlc_snapshot_sweep_consecutive_failures: u32,
    pub(crate) crypto_fiat_quote_usd: bool,
    pub(crate) crypto_fiat_quote_usdt: bool,
    pub(crate) crypto_fiat_quote_usdc: bool,
    pub(crate) crypto_fiat_quote_usdg: bool,
    pub(crate) crypto_fiat_quote_eur: bool,
    pub(crate) crypto_fiat_quote_gbp: bool,
    pub(crate) crypto_fiat_quote_cad: bool,
    pub(crate) crypto_fiat_quote_aud: bool,
    pub(crate) crypto_fiat_quote_jpy: bool,
    pub(crate) crypto_fiat_quote_chf: bool,

    /// Finnhub API key.
    pub(crate) finnhub_key: String,
    /// FRED (Federal Reserve Economic Data) API key.
    pub(crate) fred_key: String,
    /// Notification config: Discord webhook, Pushover token/user, ntfy topic.
    pub(crate) discord_webhook: String,
    pub(crate) pushover_token: String,
    pub(crate) pushover_user: String,
    pub(crate) ntfy_topic: String,
    /// AI chat (Anthropic Claude / OpenAI GPT).
    pub(crate) anthropic_key: String,
    pub(crate) openai_key: String,
    pub(crate) gemini_key: String,
    pub(crate) xai_key: String, // Grok (xAI)
    pub(crate) mistral_key: String,
    pub(crate) perplexity_key: String,
    pub(crate) show_ai_chat: bool,
    /// Claude Code CLI chat window.
    pub(crate) show_claude_code: bool,
    pub(crate) claude_code_input: String,
    pub(crate) claude_code_history: Vec<(bool, String)>, // (is_user, message)
    pub(crate) claude_code_rx: Option<std::sync::mpsc::Receiver<String>>,
    /// Research packet stored verbatim so follow-ups in the chat window still see
    /// the TyphooN fundamentals — not just the `[Research packet: AAPL]` placeholder.
    pub(crate) claude_code_packet: Option<String>,
    /// Per-session UUID (reused across Send clicks so Claude CLI resumes the same thread).
    pub(crate) claude_code_session_id: Option<String>,
    /// Picked model alias for Claude CLI: "opus" | "sonnet" | "haiku".
    pub(crate) claude_model: String,
    /// Extended-thinking trigger prepended to prompts. Stored as the literal
    /// trigger phrase ("ultrathink" | "think harder" | "think hard" | "think" | "")
    /// so we can just inject it verbatim in `build_claude_prompt`.
    pub(crate) claude_effort: String,
    /// Gemini CLI chat window.
    pub(crate) show_gemini_cli: bool,
    pub(crate) gemini_cli_input: String,
    pub(crate) gemini_cli_history: Vec<(bool, String)>,
    pub(crate) gemini_cli_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub(crate) gemini_cli_packet: Option<String>,
    pub(crate) gemini_model: String,
    /// Codex CLI chat window (local `codex` binary — OpenAI Codex).
    pub(crate) show_codex_cli: bool,
    pub(crate) codex_cli_input: String,
    pub(crate) codex_cli_history: Vec<(bool, String)>,
    pub(crate) codex_cli_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub(crate) codex_cli_packet: Option<String>,
    pub(crate) codex_model: String,
    pub(crate) codex_reasoning_effort: String,
    /// Hermes Agent CLI chat window (local `hermes` binary).
    pub(crate) show_hermes_cli: bool,
    pub(crate) hermes_cli_input: String,
    pub(crate) hermes_cli_history: Vec<(bool, String)>,
    pub(crate) hermes_cli_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub(crate) hermes_cli_packet: Option<String>,
    pub(crate) hermes_cli_session_id: String,
    /// Optional Hermes model override. Empty means use Hermes' configured default.
    pub(crate) hermes_model: String,
    /// Optional Hermes provider override. Empty means use Hermes' configured default.
    pub(crate) hermes_provider: String,
    /// Grok Build CLI chat window (local `grok` binary).
    pub(crate) show_grok_cli: bool,
    pub(crate) grok_cli_input: String,
    pub(crate) grok_cli_history: Vec<(bool, String)>,
    pub(crate) grok_cli_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub(crate) grok_cli_packet: Option<String>,
    pub(crate) grok_cli_session_id: String,
    pub(crate) grok_model: String,
    pub(crate) grok_effort: String,
    pub(crate) ai_chat_history: Vec<(bool, String)>, // (is_user, message)
    pub(crate) ai_chat_input: String,
    pub(crate) ai_chat_packet: Option<String>,
    /// Currently selected model name for the ai_provider picker. Resets to the
    /// provider default when the user switches providers.
    pub(crate) ai_model: String,
    pub(crate) ai_provider: usize, // 0=Claude, 1=GPT
    // ── AI session persistence ──
    /// Stable per-conversation UUIDs. Empty until the first turn, then reused
    /// for every subsequent save of the same conversation. Claude reuses the
    /// existing `claude_code_session_id` (a UUID) as both the --resume id and
    /// the kv-cache session id.
    pub(crate) ai_chat_session_id: String,
    pub(crate) gemini_cli_session_id: String,
    pub(crate) codex_cli_session_id: String,
    /// AI Sessions history browser.
    pub(crate) show_ai_sessions: bool,
    pub(crate) ai_sessions_index: Vec<typhoon_engine::core::ai_sessions::SessionIndexEntry>,
    pub(crate) ai_sessions_viewing: Option<typhoon_engine::core::ai_sessions::AiSessionRecord>,
    pub(crate) ai_sessions_last_refresh: i64,
    /// AI response cache stats window.
    pub(crate) show_ai_cache: bool,
    pub(crate) ai_cache_stats: typhoon_engine::core::ai_response_cache::AiResponseCacheStats,
    pub(crate) ai_cache_recent: Vec<typhoon_engine::core::ai_response_cache::AiResponseCacheEntry>,
    pub(crate) ai_cache_last_refresh: i64,
    /// Reddit WSB posts.
    pub(crate) show_reddit: bool,
    /// BARDATA sync progress tracking.
    pub(crate) show_bardata: bool,
    pub(crate) bardata_total: usize,
    pub(crate) bardata_queued: usize,
    pub(crate) bardata_completed: usize,
    pub(crate) bardata_skipped: usize,
    pub(crate) bardata_log: VecDeque<String>,
    pub(crate) bardata_active: bool,
    pub(crate) reddit_posts: Vec<(String, String, u64, u64)>, // (title, url, score, comments)
    /// Matrix chat (community chat room — send + receive).
    pub(crate) show_matrix_chat: bool,
    pub(crate) matrix_room: String,
    pub(crate) matrix_messages: Vec<(String, String, String)>, // (sender, timestamp, body)
    pub(crate) matrix_input: String,
    pub(crate) matrix_access_token: String,
    pub(crate) matrix_user_id: String,
    pub(crate) matrix_last_fetch: std::time::Instant,
    /// Legacy cached Finnhub news tuples (headline, source, datetime) — used by the
    /// compact "News" side-pane and the WASM web mirror. Retained for backward compat.
    pub(crate) news_articles: Vec<(String, String, String)>,
    /// Rich multi-source news for the two-pane NEWS reader.
    pub(crate) news_full_articles: Vec<typhoon_engine::core::news::NewsArticle>,
    /// Index into `news_full_articles` currently open in the right pane.
    pub(crate) news_selected: Option<usize>,
    /// Symbol currently loaded into the news window (drives refresh button).
    pub(crate) news_symbol_filter: String,
    /// Full-text search query for the news reader.
    pub(crate) news_search_query: String,
    /// URL hash of the article selected before the most recent reload/session restore.
    pub(crate) news_selected_url_hash: String,
    /// UI state flag while a fetch/cached-load is in flight.
    pub(crate) news_loading: bool,
    /// Watchdog start time for news_loading so a lost broker result cannot keep heavy-sync mode latched forever.
    pub(crate) news_loading_started_at: Option<std::time::Instant>,
    /// Content hash of the current news_full_articles set (used for cache guard / re-filter decisions, mirroring the pattern requested for fundamentals).
    pub(crate) news_input_hash: u64,
    /// Latches once the News window has triggered its initial
    /// `LoadCachedNews` for the session. Prevents auto-load from
    /// firing every frame the window stays open, while still
    /// re-triggering on the next restart.
    pub(crate) news_initial_load_done: bool,
    /// Total rows in the `research_news` SQLite table, pushed from the broker
    /// via `BrokerMsg::NewsDbTotal` after each cache load / fresh fetch / scope
    /// scrape so the header shows "· N in DB" even when the in-memory list is
    /// empty. `None` until the first push arrives. The render thread never
    /// queries this itself (the old poll blocked on the write mutex).
    pub(crate) news_db_total: Option<i64>,
    /// User-entered Marketaux API key (free tier 100/day).
    pub(crate) marketaux_key: String,
    /// User-entered Alpha Vantage API key (free tier 25/day).
    pub(crate) alpha_vantage_key: String,
    /// User-entered FMP API key (free tier 250/day, also used for transcripts).
    pub(crate) fmp_key: String,
    /// User-entered CryptoPanic API token (free public tier, per-currency filter).
    pub(crate) cryptopanic_key: String,

    /// SL/TP planning lines (visual, pre-broker).
    pub(crate) sl_price: Option<f64>,
    pub(crate) tp_price: Option<f64>,
    /// True while user is dragging the SL line on the chart.
    pub(crate) dragging_sl: bool,
    /// True while user is dragging the TP line on the chart.
    pub(crate) dragging_tp: bool,

    // ── risk calculator state ────────────────────────────────────────────
    pub(crate) rc_equity: String,
    pub(crate) rc_risk_pct: String,
    pub(crate) rc_entry: String,
    pub(crate) rc_sl: String,
    pub(crate) rc_tp: String,
    pub(crate) rc_tick_value: String,
    pub(crate) rc_tick_size: String,
    pub(crate) rc_result: String,

    // ── backtest state ───────────────────────────────────────────────────
    pub(crate) bt_strategy: usize,
    pub(crate) bt_fast_period: String,
    pub(crate) bt_slow_period: String,
    pub(crate) bt_equity: String,
    pub(crate) bt_result: Option<backtest::TradeReport>,
    pub(crate) bt_trades: Vec<backtest::Trade>,
    pub(crate) bt_equity_curve: Vec<f64>,

    // ── optimizer state ──────────────────────────────────────────────────
    pub(crate) opt_fast_range: String,
    pub(crate) opt_slow_range: String,
    pub(crate) opt_results: Vec<backtest::OptimizationResult>,
    // Walk-forward analysis state
    pub(crate) wf_result: Option<backtest::WalkForwardResult>,
    pub(crate) wf_windows_count: String,
    // GPU optimizer state
    pub(crate) opt_rsi_range: String,
    pub(crate) opt_atr_sl_range: String,
    pub(crate) opt_atr_tp_range: String,
    pub(crate) gpu_opt_results: Vec<gpu_compute::BacktestResult>,
    pub(crate) gpu_opt_combos: Vec<gpu_compute::ParamCombo>,
    pub(crate) gpu_backtester: Option<gpu_compute::GpuBacktester>,

    // ── margin monitor state ─────────────────────────────────────────────
    pub(crate) mm_equity: String,
    pub(crate) mm_margin: String,
    pub(crate) mm_margin_per_lot: String,
    pub(crate) mm_trim_pct: String,
    pub(crate) mm_result: String,

    // ── tab bar ──────────────────────────────────────────────────────────
    /// Index of the active tab (into `charts`).
    pub(crate) active_tab: usize,

    // ── watchlist ────────────────────────────────────────────────────────
    /// Rich watchlist data: symbol name, last, prev_close, change, change_pct, volume, cache_key.
    pub(crate) watchlist_rows: Vec<WatchlistRow>,
    /// Unix timestamp of last successful watchlist quote refresh — drives staleness badge.
    pub(crate) watchlist_last_update_ts: i64,
    /// Last time an automatic watchlist quote refresh was dispatched. `None` until
    /// the first dispatch, so the watchlist populates on startup rather than only
    /// when the user manually adds a symbol. See the periodic refresh in `update`.
    pub(crate) watchlist_auto_refresh_at: Option<std::time::Instant>,
    /// Watchlist length at the last quote fetch. A change forces an immediate
    /// refresh even on the slow weekend cadence (new symbol added / removed).
    pub(crate) watchlist_quotes_fetched_count: usize,
    /// Unix timestamp of last successful positions refresh.
    pub(crate) positions_last_update_ts: i64,
    /// Last automatic positions/orders snapshot dispatch. Separate from successful
    /// update timestamp so a slow broker response does not cause per-frame request spam.
    pub(crate) positions_auto_refresh_at: Option<std::time::Instant>,
    /// Unix timestamp of last successful orders refresh.
    pub(crate) orders_last_update_ts: i64,
    /// User-managed watchlist symbols (persisted in session).
    pub(crate) user_watchlist: Vec<String>,
    /// PERF: O(1) membership for watchlist (watchlist display preserves Vec order).
    /// Maintained in sync with user_watchlist. Replaces linear .contains on Vec.
    pub(crate) user_watchlist_set: std::collections::HashSet<String>,
    /// Fallback prices from Yahoo (or other sources) when primary broker has no data.
    /// Key = symbol, Value = (price, source, timestamp)
    pub(crate) watchlist_fallback_prices:
        std::collections::HashMap<String, (f64, String, std::time::Instant)>,
    /// Input field for adding symbols to watchlist.
    pub(crate) watchlist_input: String,

    // ── floating window visibility ───────────────────────────────────────
    pub(crate) show_settings: bool,
    pub(crate) was_settings_open: bool,
    pub(crate) show_risk_calc: bool,
    pub(crate) show_compound_calc: bool,
    pub(crate) ci_principal: String,
    pub(crate) ci_rate: String,
    pub(crate) ci_years: String,
    pub(crate) ci_compounds: String,
    pub(crate) ci_contribution: String,
    pub(crate) ci_result: Vec<(f64, f64, f64)>,
    pub(crate) show_backtest: bool,
    pub(crate) show_screener: bool,
    pub(crate) screener_filter: String,
    pub(crate) screener_sort_col: usize,
    pub(crate) screener_sort_asc: bool,
    pub(crate) show_symbols: bool,
    pub(crate) symbols_filter: String,
    pub(crate) symbols_expanded: std::collections::HashSet<String>,
    /// Full asset list from broker (symbol, name, asset_class) for Symbol Explorer.
    pub(crate) all_broker_assets: Vec<(String, String, String)>,
    pub(crate) all_broker_assets_fetched: bool,
    pub(crate) show_optimizer: bool,
    pub(crate) show_news: bool,
    pub(crate) show_calendar: bool,
    pub(crate) show_sec: bool,
    pub(crate) sec_selected_filing: Option<usize>,
    pub(crate) sec_tab: usize, // 0=Filings, 1=Alerts, 2=Insiders, 3=Timeline
    pub(crate) sec_search_query: String, // text search filter for filings
    pub(crate) sec_keyword_input: String, // keyword watchlist input
    pub(crate) sec_keywords: Vec<String>, // cached keyword list
    pub(crate) earnings_active_only: bool, // filter earnings calendar to active symbols
    pub(crate) dividends_active_only: bool, // filter dividend calendar to active symbols
    pub(crate) ev_active_only: bool, // filter EV scanner to active symbols
    /// Per-symbol visibility toggles in the Fundamentals window.
    pub(crate) fundamentals_hidden_symbols: std::collections::HashSet<String>,
    pub(crate) congress_active_only: bool, // filter congressional trades to active symbols
    pub(crate) volume_active_only: bool,   // filter unusual volume to active symbols
    pub(crate) sec_filing_content: String, // cached filing document text
    pub(crate) sec_filing_content_for: String, // accession number this content belongs to (for sticky display)
    pub(crate) sec_filing_pinned: bool, // pin document viewer (don't clear when navigating filings)
    pub(crate) sec_filing_loading: bool,
    pub(crate) sec_filing_summary: Option<sec_filing::FilingSummary>,
    pub(crate) sec_filing_summary_for: String,
    pub(crate) show_insider: bool,
    pub(crate) show_fundamentals: bool,
    pub(crate) show_analyst: bool,
    pub(crate) analyst_result: String, // last fetched Finnhub recommendations JSON
    pub(crate) show_holders: bool,
    pub(crate) holders_result: String, // last fetched SEC EDGAR 13F JSON
    pub(crate) show_orderbook_window: bool,
    pub(crate) orderbook_result: String, // last fetched L2 orderbook JSON
    pub(crate) kraken_orderbook_ws_symbol: String,
    pub(crate) kraken_chart_l2_ws_symbol: String,
    pub(crate) kraken_chart_l2_last_start_attempt: std::time::Instant,
    pub(crate) market_clock_status: String,
    pub(crate) show_seasonals: bool,
    pub(crate) show_montecarlo: bool,
    pub(crate) show_stress_test: bool,
    pub(crate) show_volume_profile: bool,
    pub(crate) show_hv_cone: bool,
    pub(crate) show_sector_heatmap: bool,
    pub(crate) show_dividends: bool,
    pub(crate) show_company_info_window: bool,
    pub(crate) company_info_symbol: String,
    pub(crate) company_info_text: String,
    /// Global broker scope filter applied to all fundamental-based commands
    /// (OUTLIERS, EVOUTLIERS, DIVSCREEN, SECTOR_HEATMAP, HV_CONE, EV viewer, etc.).
    /// All = no filter. Use `SCOPE [ALL|ALPACA|KRAKEN]` command to change.
    pub(crate) broker_scope: EventSource,
    /// Cached broker scope HashSet. Invalidated by `(bg_rev, broker_scope)` pair —
    /// only recomputed when fundamentals/specs load (bg_rev bumped) or scope changes.
    /// O(1) reads for the 5+ windows that need scope filtering.
    pub(crate) cached_scope_syms: Option<std::collections::HashSet<String>>,
    pub(crate) cached_scope_key: Option<(u64, EventSource)>,
    /// Monotonic counter bumped each time `self.bg` is replaced from the BG thread.
    /// Used as a dirty-flag for any cache derived from `bg.*` state.
    pub(crate) bg_rev: u64,
    /// UX6: One-shot flag to auto-scroll outlier table to first EXTREME tier on next render.
    pub(crate) outlier_scroll_pending: bool,
    /// UX4: Named workspace presets — maps name → JSON snapshot of show_* flags.
    pub(crate) workspaces: std::collections::HashMap<String, String>,
    /// UX7: Sparkline cache — last 30 daily closes per symbol, lazy-fetched on first render.
    /// PERF: values wrapped in Arc so `get_sparkline` returns O(1) clones instead of copying
    /// the whole Vec<f64> on every cache hit (called for every visible window row per frame).
    pub(crate) sparkline_cache: std::collections::HashMap<String, std::sync::Arc<Vec<f64>>>,
    /// UX3: Deferred symbol action from right-panel context menus (applied at end of update()).
    pub(crate) deferred_symbol_action: SymbolAction,
    /// PERF: Cached active symbols list. Recomputed when chart/position/watchlist inputs change.
    pub(crate) cached_active_symbols: Vec<String>,
    pub(crate) cached_active_symbols_key: Option<u64>,
    /// PERF: HashSet of cached_active_symbols for O(1) lookup.
    pub(crate) cached_active_symbols_set: std::collections::HashSet<String>,
    /// Cached scoped fundamentals (filtered by broker_scope). Rebuilt only when
    /// `(bg_rev, broker_scope)` changes — not per frame. Used by Sector Heatmap,
    /// Dividend Yield Screener, Outlier Scanner.
    pub(crate) cached_scoped_fundamentals: Vec<typhoon_engine::core::fundamentals::Fundamentals>,
    pub(crate) cached_scoped_fundamentals_key: Option<(u64, EventSource)>,
    /// Cached Alpaca bar-state map (symbol, timeframe) -> sync metadata.
    /// Rebuilt only when `bg_rev` changes so the sync scheduler doesn't rescan
    /// `bg.detailed_stats` every rotation.
    pub(super) cached_alpaca_sync_state:
        std::collections::HashMap<(String, String), SyncCacheState>,
    pub(crate) cached_alpaca_sync_state_rev: Option<u64>,
    /// Cached Kraken bar-state map keyed by normalized `(symbol, timeframe)`.
    pub(super) cached_kraken_sync_state:
        std::collections::HashMap<(String, String), SyncCacheState>,
    pub(crate) cached_kraken_sync_state_rev: Option<u64>,
    /// Memoized normalized Kraken equity catalog (the ~12k xStock universe), so the
    /// 60s scheduler doesn't re-normalize+sort the whole list every tick. Signature
    /// is `kraken_equity_universe_symbols.len()` — the list is replaced wholesale on
    /// reload, and a same-length replacement normalizes to the same result anyway.
    pub(super) cached_kraken_equity_catalog: Vec<String>,
    pub(crate) cached_kraken_equity_catalog_sig: Option<usize>,
    /// Memoized Alpaca equity rotation list (~11k us_equity assets + a chart/
    /// watchlist floor), so the 60s rotation doesn't uppercase+dedup+sort the whole
    /// universe every tick. Signature is the three input lengths; the chart/watchlist
    /// floor is a backup (those symbols also sync via demand), so one cycle of
    /// staleness on a rare same-length swap is harmless.
    pub(super) cached_alpaca_equity_rotation: Vec<String>,
    pub(crate) cached_alpaca_equity_rotation_sig: Option<(usize, usize, usize)>,
    /// Cached Kraken Futures bar-state map keyed by normalized `(symbol, timeframe)`.
    pub(super) cached_kraken_futures_sync_state:
        std::collections::HashMap<(String, String), SyncCacheState>,
    pub(crate) cached_kraken_futures_sync_state_rev: Option<u64>,
    /// Cached Kraken internal equities bar-state map keyed by normalized `(symbol, timeframe)`.
    pub(super) cached_kraken_equities_sync_state:
        std::collections::HashMap<(String, String), SyncCacheState>,
    pub(crate) cached_kraken_equities_sync_state_rev: Option<u64>,
    /// Cached Yahoo Chart assist bar-state map keyed by normalized `(symbol, timeframe)`.
    pub(super) cached_yahoo_chart_sync_state:
        std::collections::HashMap<(String, String), SyncCacheState>,
    pub(crate) cached_yahoo_chart_sync_state_rev: Option<u64>,
    /// Cached Sync Status rows. The window is informational; recomputing the
    /// whole broker/timeframe matrix on every repaint during full sync is pure
    /// render-thread waste.
    pub(super) cached_bar_sync_rows: Vec<SyncStatsRow>,
    pub(crate) cached_bar_sync_rows_last: std::time::Instant,
    /// Receiver for an in-flight bar-sync matrix recompute running on a blocking
    /// worker. The full xStocks/Merged scan is hundreds of ms of CPU on a 12k
    /// universe, so it is never run on the render thread; `Some` means a compute
    /// is in flight and a new one must not be dispatched.
    pub(crate) bar_sync_compute_rx:
        Option<std::sync::mpsc::Receiver<super::sync_status::BarSyncResult>>,
    /// Coverage % across live brokers from the most recent Sync Status snapshot.
    /// Refreshed on every `compute_bar_sync_rows` call (cached, ≤1Hz).
    /// `auto_full_tilt_until_caught_up` consults it to keep request pressure
    /// high while the catch-up is in progress and let the scheduler drop back
    /// to balanced cadence once it crosses the healthy threshold.
    pub(crate) cached_bar_sync_overall_pct: f32,
    /// Latched "catch-up in progress" flag. Engages when overall coverage drops
    /// below the engage threshold and only releases once we cross the higher
    /// release threshold — hysteresis to keep a row flipping at the edge from
    /// rapidly cycling full-tilt mode on/off.
    pub(crate) auto_full_tilt_active: bool,
    /// Full tradable Kraken Securities/equities symbol universe from the internal public catalog.
    pub(crate) kraken_equity_universe_symbols: Vec<String>,
    /// O(1) membership mirror for `kraken_equity_universe_symbols`; used to keep
    /// unsupported Alpaca/Yahoo low-timeframe assist fetches from repeatedly
    /// probing xStock rows that the native Kraken-equities path owns.
    pub(crate) kraken_equity_universe_set: std::collections::HashSet<String>,
    /// Lightweight ticker → company name map populated from the Kraken iapi equity
    /// catalog at universe load time. Used as a fast fallback for chart headers and
    /// the symbol picker when the full `all_fundamentals` table is empty (the normal
    /// case for the 12 k xStock universe because the heavy scrape is intentionally
    /// deferred).
    pub(crate) kraken_equity_names: std::collections::HashMap<String, String>,
    /// WS-tokenized xStock subset of the universe (the `{SYM}x/USD` pairs that
    /// actually exist on Kraken's public WS v2 OHLC channel, ~150 of the ~12k).
    /// Scopes the WS OHLC snapshot sweep; the full catalog still syncs via the
    /// Alpaca/Yahoo breadth lanes + demand-scoped iapi.
    pub(crate) kraken_equity_tokenized_symbols: Vec<String>,
    /// Equity symbols whose iapi `overnight_trading_support` is disabled — they
    /// trade pre/core/after only (CLOSED 8 PM–4 AM ET), not the full 24/5 cycle.
    /// Absence ⇒ overnight-enabled (the common case); drives the session label.
    pub(crate) kraken_equity_no_overnight: std::collections::HashSet<String>,
    pub(crate) kraken_equity_universe_requested: bool,
    pub(crate) show_reg_sho_window: bool,
    /// Floating window listing current trading halts / LULD pauses (parallels the
    /// Reg SHO window). Opened by the HALTS command.
    pub(crate) show_halts_window: bool,
    /// Cache-loaded price snapshots (last / prev close / chg%) for regulatory
    /// alert symbols (Reg SHO threshold OR trading halt) not already in the
    /// watchlist, so the Reg SHO and Halts windows can fill their price columns
    /// for every listed symbol — keyed by the normalized alert symbol. Populated
    /// off the render thread when either window opens.
    pub(crate) regulatory_prices: std::collections::HashMap<String, WatchlistRow>,
    /// Receiver for the async Reg SHO price load (off the render thread to avoid
    /// the SQLite-read stall when a bulk bar-sync writer holds the conn mutex).
    pub(crate) regulatory_prices_rx: Option<std::sync::mpsc::Receiver<Vec<(String, WatchlistRow)>>>,
    /// Guards the one-time staleness-ordered fetch kick per window open (reset
    /// when both windows close). The cached-price *read* re-runs on a throttle
    /// (`regulatory_price_read_at`) so freshly fetched bars keep surfacing.
    pub(crate) regulatory_prices_loaded: bool,
    /// Throttle anchor for the periodic off-thread regulatory price re-read while
    /// either window is open; `None` forces an immediate read next frame.
    pub(crate) regulatory_price_read_at: Option<std::time::Instant>,
    pub(crate) reg_sho_sort: Option<(usize, bool)>,
    pub(crate) halts_sort: Option<(usize, bool)>,
    pub(crate) kraken_equity_universe_retry_after_ts: i64,
    pub(crate) kraken_equities_sync_pause_until_ts: i64,
    pub(crate) kraken_equities_sync_pause_reason: String,
    pub(crate) yahoo_chart_sync_pause_until_ts: i64,
    pub(crate) yahoo_chart_sync_pause_reason: String,
    pub(crate) heavy_sync_in_progress: bool,
    /// SEC window caches — all keyed off `(bg_rev, broker_scope, ...)` so the heavy
    /// dedup/filter/sort work only runs when state actually changes, not every frame.
    /// Keys are u64 hashes for zero-alloc per-frame comparison.
    /// Filings tab: sorted indices into `bg.sec_filings` after dedup+scope+filter+search.
    pub(crate) sec_cache_filings: Vec<usize>,
    pub(crate) sec_cache_filings_key: Option<u64>,
    /// User-controlled filter/search/sort key for the filings cache. Kept
    /// separate from the SEC data key so checkbox/search changes can rebuild
    /// the visible table even while a broad EDGAR scrape is publishing rows.
    pub(crate) sec_cache_filings_controls_key: Option<u64>,
    /// Insiders tab: (ticker, trade index) tuples for cross-symbol rendering.
    pub(crate) sec_cache_insiders: Vec<(String, usize)>,
    pub(crate) sec_cache_insiders_clusters: Vec<(String, usize)>,
    pub(crate) sec_cache_insiders_key: Option<u64>,
    /// Timeline tab: (month, count, "type:count type:count..." breakdown) per month, newest first.
    pub(crate) sec_cache_timeline: Vec<(String, usize, String)>,
    pub(crate) sec_cache_timeline_key: Option<u64>,
    /// Tab count strings — `(scoped_filings, alerts, insider_total)`.
    pub(crate) sec_cache_tab_counts: (usize, usize, usize),
    pub(crate) sec_cache_tab_counts_key: Option<u64>,
    /// Last time SEC window caches performed O(N) rebuild work on the UI thread.
    pub(crate) sec_cache_last_rebuild: std::time::Instant,
    pub(crate) show_event_calendar: bool,
    pub(crate) event_calendar_rows: Vec<EventRow>,
    pub(crate) event_filter_source: EventSource,
    pub(crate) event_filter_earnings: bool,
    pub(crate) event_filter_exdiv: bool,
    pub(crate) event_filter_divpay: bool,
    pub(crate) show_confluence: bool,
    pub(crate) show_stat_arb: bool,
    pub(crate) show_risk_budget: bool,
    pub(crate) show_order_flow: bool,
    pub(crate) order_flow_footprint_bars: usize,
    pub(crate) show_bookmap: bool,
    pub(crate) bookmap_windows: Vec<BookmapWindowState>,
    pub(crate) show_outliers: bool,
    pub(crate) outliers: Vec<typhoon_engine::core::var::OutlierResult>,
    pub(crate) sector_stats: Vec<typhoon_engine::core::var::SectorStats>,
    pub(crate) multi_outliers: Vec<typhoon_engine::core::var::MultiOutlierResult>,
    pub(crate) show_option_chain: bool,
    pub(crate) option_chain_sym: String, // symbol last fetched
    // MQL5/PineScript/…/transpile compiler
    pub(crate) show_indicator_compiler: bool,
    pub(crate) compiler_source: String,  // source code input
    pub(crate) compiler_language: usize, // see COMPILER_LANGS below
    pub(crate) compiler_transpile_target: usize, // target language index for transpile dropdown
    pub(crate) compiler_transpiled: Option<String>, // transpiled source output
    pub(crate) compiler_diagnostics: VecDeque<String>,
    pub(crate) compiler_metadata: Option<typhoon_transpiler::CompileResult>,
    pub(crate) show_journal: bool,
    pub(crate) show_object_list: bool,
    pub(crate) journal_entries: Vec<JournalEntry>,
    pub(crate) show_var_mult: bool,
    pub(crate) show_margin_monitor: bool,
    pub(crate) show_cache_stats: bool,
    pub(crate) show_storage: bool,
    pub(crate) storage_filter: String,
    pub(crate) storage_delete_confirm: Option<String>,
    pub(crate) storage_delete_filtered_confirm: bool,
    pub(crate) storage_prune_disabled_kraken_quotes_confirm: bool,
    pub(crate) storage_purge_bars_confirm: bool,
    pub(crate) storage_purge_broker_confirm: Option<String>,
    pub(crate) storage_purge_timeframe_confirm: bool,
    /// Slider position for the news age-purge tool (index into the
    /// NEWS_PURGE_AGE_DAYS notch array — 7/30/90/180/365/730/1825).
    /// Default 4 = 1 year, which is the most common "I want to free
    /// some space but keep recent context" pick.
    pub(crate) storage_purge_news_age_idx: usize,
    /// 2-step confirmation latch for the news purge button.
    pub(crate) storage_purge_news_confirm: bool,
    pub(crate) storage_page: usize,
    pub(crate) storage_sort_col: usize,
    pub(crate) storage_sort_asc: bool,
    /// Cached filtered/sorted Storage Manager rows. Rebuilding this every frame
    /// against a multi-million-row cache summary causes hard UI stalls.
    pub(crate) storage_filtered_rows_cache: Vec<(String, i64, i64)>,
    pub(crate) storage_filtered_rows_cache_key: Option<u64>,
    pub(crate) storage_disabled_kraken_quote_keys_cache: Vec<String>,
    pub(crate) storage_disabled_kraken_quote_keys_cache_rev: Option<u64>,
    /// Broader "out-of-scope" Kraken purge: confirm latch + cached key set
    /// (keys the current sync config would not fetch — disabled sector or no
    /// longer in Kraken's loaded universe). Mirrors the disabled-quote prune.
    pub(crate) storage_prune_out_of_scope_kraken_confirm: bool,
    pub(crate) storage_out_of_scope_kraken_keys_cache: Vec<String>,
    pub(crate) storage_out_of_scope_kraken_keys_cache_rev: Option<u64>,
    pub(crate) cache_stats_sort_col: usize,
    pub(crate) cache_stats_sort_asc: bool,
    /// Canonical cache suffix selected for "delete this timeframe across all brokers".
    pub(crate) storage_delete_timeframe: String,
    /// Text buffer for the "new cache location" input in Storage Manager. Holds
    /// the path the user is typing BEFORE they click Save/Copy. Seeded with
    /// the currently-configured custom dir (if any) when Storage Manager opens.
    pub(crate) storage_cache_path_input: String,
    /// Last result from the Save / Copy / Reset cache-location action. Shown
    /// inline in Storage Manager. `(success, message)`.
    pub(crate) storage_cache_move_result: Option<(bool, String)>,
    /// Live async result channel for "Copy cache to new location (VACUUM INTO)".
    /// Produced by a worker thread, consumed once in the render loop to
    /// populate `storage_cache_move_result`. `None` = no op in flight.
    pub(crate) storage_cache_move_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    /// Bar-sync % per broker/TF window. Counted from `bg.detailed_stats`,
    /// with freshness derived from `bg.bar_ts_cache` when available.
    pub(crate) show_sync_status: bool,
    pub(crate) show_help: bool,
    pub(crate) help_filter: String,
    /// Count of alerts that have fired since the user last dismissed the badge.
    /// Rendered as a red breach counter on the top bar so the trader can't miss it.
    pub(crate) alert_breach_count: u32,
    /// Unix timestamp of the most recent alert breach (for tooltip display).
    pub(crate) alert_last_breach_ts: i64,
    /// Message of the most recent alert (shown in the breach tooltip).
    pub(crate) alert_last_breach_msg: String,
    /// Deduplicate broker bar fetches per source. Keys are normalized `SYMBOL:TF`.
    pub(crate) pending_alpaca_fetches: std::collections::HashSet<String>,
    pub(crate) pending_kraken_fetches: std::collections::HashSet<String>,
    pub(crate) pending_kraken_futures_fetches: std::collections::HashSet<String>,
    pub(crate) pending_yahoo_chart_fetches: std::collections::HashSet<String>,
    /// Per-key cooldown for broker bar re-queues. The in-flight HashSet only
    /// dedups while a fetch is pending; once it completes we'd previously
    /// re-queue immediately on the next sync tick, which (during a closed
    /// market) hit the same SYMBOL:TF every minute without producing new
    /// bars. Keys are `{source}:{SYMBOL}:{TF}` and values are the unix-second
    /// timestamp of the last queued fetch. A re-queue within ~half the TF
    /// period is skipped.
    pub(crate) fetch_last_queued_ts: std::collections::HashMap<String, i64>,
    /// Cursor-limited broad sync rotation. Each refill scans only a bounded slice
    /// of the broker universe in high-timeframe-first order, while the pending
    /// fetch sets keep foreground/manual and background requests deduplicated.
    ///
    /// Keep independent cursors for native Kraken Equities/iapi and the fast
    /// Alpaca/Yahoo assist lanes. Sharing the iapi cursor made Cloudflare-bound
    /// native repair, Alpaca batches, and Yahoo requests advance one another's
    /// symbol windows, slowing broad Merged coverage convergence.
    pub(crate) alpaca_sync_cursor: usize,
    pub(crate) kraken_spot_sync_cursors: [usize; 4],
    pub(crate) kraken_equities_sync_cursor: usize,
    pub(crate) kraken_equities_alpaca_sync_cursor: usize,
    pub(crate) yahoo_chart_sync_cursor: usize,
    pub(crate) kraken_futures_sync_cursors: [usize; 4],
    /// Alpaca retry queue — persisted across restarts via cache KV at `alpaca:retry_queue`.
    /// Entries are (symbol, timeframe) pairs that 429'd or partially completed; the
    /// `poll_alpaca_retry_queue()` tick re-dispatches due entries with exponential backoff.
    pub(crate) alpaca_retry_queue: Vec<AlpacaRetry>,
    /// Unix seconds of the last retry-queue poll tick (throttled to 10s intervals).
    pub(crate) alpaca_retry_last_poll: i64,
    /// Set once after startup to trigger the initial KV load into `alpaca_retry_queue`.
    pub(crate) alpaca_retry_loaded: bool,
    /// First time the persisted Alpaca retry queue diverged from memory.
    /// Flushed in coarse batches; never write this KV on every worker result.
    pub(crate) alpaca_retry_dirty_since: Option<std::time::Instant>,
    /// Definitive no-data tombstones for Alpaca symbol/timeframe pairs.
    /// Persisted via cache KV at `alpaca:no_data_pairs` and consulted by all
    /// automated scheduling paths before dispatch.
    pub(super) alpaca_no_data_pairs: std::collections::HashMap<String, AlpacaNoDataPair>,
    pub unresolvable_pairs: std::collections::HashMap<String, UnresolvablePair>,
    /// Per-broker normalized `SYMBOL:TF` tombstone indexes for O(1) scheduler
    /// membership checks without rebuilding a filtered HashSet on every refill.
    pub(crate) unresolvable_fetch_keys_by_broker:
        std::collections::HashMap<String, std::collections::HashSet<String>>,
    pub(crate) alpaca_no_data_loaded: bool,
    /// First time the persisted Alpaca no-data tombstone set diverged from memory.
    pub(crate) alpaca_no_data_dirty_since: Option<std::time::Instant>,
    /// First time the persisted unresolvable-pair set diverged from memory.
    pub(crate) unresolvable_dirty_since: Option<std::time::Instant>,
    /// Persisted "bounded full-history fetch already exhausted available
    /// Alpaca bars for this pair" markers. Only suppresses repeat Backfill
    /// scheduling; Missing/Stale sync still proceeds normally.
    pub(super) alpaca_backfill_complete_pairs:
        std::collections::HashMap<String, AlpacaBackfillCompletePair>,
    pub(crate) alpaca_backfill_complete_loaded: bool,
    pub(crate) alpaca_backfill_complete_dirty_since: Option<std::time::Instant>,
    pub(super) kraken_backfill_complete_pairs:
        std::collections::HashMap<String, AlpacaBackfillCompletePair>,
    pub(crate) kraken_backfill_complete_loaded: bool,
    pub(crate) kraken_backfill_complete_dirty_since: Option<std::time::Instant>,
    pub(super) kraken_futures_backfill_complete_pairs:
        std::collections::HashMap<String, AlpacaBackfillCompletePair>,
    pub(crate) kraken_futures_backfill_complete_loaded: bool,
    pub(crate) kraken_futures_backfill_complete_dirty_since: Option<std::time::Instant>,
    pub(crate) show_connect: bool,
    pub(crate) show_indicators_panel: bool,
    pub(crate) show_data_window: bool,
    pub(crate) show_alerts: bool,
    // Fundamentals symbol source settings
    pub(crate) fund_source_alpaca: bool,
    pub(crate) fund_source_kraken: bool,
    /// ADR-094: SCOPE popup window with source checkboxes.
    pub(crate) show_scope_window: bool,
    // Scrape status dashboard
    pub(crate) show_scrape_status: bool,
    pub(crate) scrape_fund_running: bool,
    pub(crate) scrape_fund_started_at: Option<std::time::Instant>,
    pub(crate) scrape_fund_ok: usize,
    pub(crate) scrape_fund_fail: usize,
    pub(crate) scrape_fund_skipped: usize,
    pub(crate) scrape_fund_total: usize,
    pub(crate) scrape_fund_last_msg: String,
    /// Startup auto fundamentals was deferred because selected source universes
    /// were not loaded yet. Re-fired once the universe symbols arrive.
    pub(crate) auto_fundamentals_deferred: bool,
    pub(crate) auto_fundamentals_started: bool,
    pub(crate) scrape_sec_running: bool,
    pub(crate) scrape_sec_started_at: Option<std::time::Instant>,
    pub(crate) scrape_sec_last_msg: String,
    /// Startup auto SEC scrape was deferred because Scope had no symbols yet.
    pub(crate) auto_sec_scrape_deferred: bool,
    /// Hash-based dedup for broker KV writes — skip put_kv if content unchanged
    pub(crate) kv_write_hashes: std::collections::HashMap<&'static str, u64>,
    /// Throttle: last write time per KV key (max once per 30s even if content changes)
    pub(crate) kv_write_times: std::collections::HashMap<&'static str, std::time::Instant>,
    // Fundamentals windows
    pub(crate) show_ev_scanner: bool,
    pub(crate) show_earnings_calendar: bool,
    pub(crate) show_dividend_calendar: bool,
    // Unusual Whales / Godel Terminal features
    pub(crate) show_unusual_volume: bool,
    pub(crate) unusual_volume_results: Vec<(String, f64, f64, f64)>, // (symbol, today_vol, avg_vol, ratio)
    pub(crate) show_sector_rotation: bool,
    pub(crate) show_fred: bool,
    pub(crate) fred_data: Vec<typhoon_engine::core::fred::FredSeries>,
    pub(crate) fred_yield_curve: Vec<(String, f64)>,
    pub(crate) show_econ_calendar: bool,
    pub(crate) econ_events: Vec<(String, String, String, String, String)>, // (date, country, event, impact, actual)
    // Calendar filters — persisted across window open/close.
    pub(crate) econ_filter_high: bool,
    pub(crate) econ_filter_medium: bool,
    pub(crate) econ_filter_low: bool,
    pub(crate) econ_filter_holiday: bool,
    /// Currency/country filter text. Empty = all. Comma-separated (e.g. "USD,EUR,GBP").
    pub(crate) econ_filter_currencies: String,
    /// Unix timestamp of last successful econ calendar fetch, for staleness badge.
    pub(crate) econ_last_fetch_ts: i64,
    pub(crate) show_congress: bool,
    pub(crate) congress_trades: Vec<(String, String, String, String, String, String)>, // (date, rep, ticker, type, amount, party)
    /// SEC filing type filters [Form 4, 13F, DEF 14A, S-1, 10-K, 10-Q, 8-K].
    pub(crate) sec_filters: [bool; 7],
    /// SEC filings pagination (0-indexed page number).
    pub(crate) sec_page: usize,

    /// Sort states for data tables.
    pub(crate) ev_sort: SortState,
    pub(crate) sec_sort: SortState,
    pub(crate) insider_sort: SortState,
    pub(crate) outlier_sort: SortState,
    /// Sort state for the single-metric outlier table (VAROUTLIER/EVOUTLIER/ATROUTLIER).
    pub(crate) outlier_single_sort: SortState,
    pub(crate) watchlist_sort: SortState,
    /// Whether we've already tried populating watchlist from cache (avoid repeated DB scans).
    pub(crate) watchlist_cache_tried: bool,

    /// Price alerts.
    pub(crate) alerts: Vec<(f64, String)>,
    /// O(1) dedup/lookup for price alerts (key "price:8|label").
    pub(crate) alerts_set: std::collections::HashSet<String>,
    pub(crate) alert_price_input: String,
    // Indicator-based alert engine
    pub(crate) indicator_alerts: Vec<IndicatorAlert>,
    /// O(1) for indicator alerts (key "sym:tf:ind:cond:thresh").
    pub(crate) indicator_alerts_set: std::collections::HashSet<String>,
    pub(crate) show_alert_builder: bool,
    pub(crate) alert_symbol: String,
    pub(crate) alert_indicator: usize, // index into ALERT_INDICATORS
    pub(crate) alert_condition: usize, // 0=crosses above, 1=crosses below, 2=greater than, 3=less than
    pub(crate) alert_threshold: String,
    // Risk-of-Ruin
    pub(crate) show_risk_ruin: bool,
    pub(crate) ruin_win_rate: String,
    pub(crate) ruin_avg_win: String,
    pub(crate) ruin_avg_loss: String,
    pub(crate) ruin_risk_pct: String,
    pub(crate) ruin_results: Vec<f32>, // final equity per simulation
    // Replay mode
    pub(crate) replay_active: bool,
    pub(crate) replay_bar_idx: usize,
    pub(crate) replay_playing: bool,
    pub(crate) replay_speed: f32,
    pub(crate) replay_timer: f32,
    // Symbol autocomplete
    pub(crate) symbol_suggestions: Vec<(String, String, String)>, // (symbol, company, sector)
    pub(crate) symbol_ac_selected: usize,
    pub(crate) symbol_ac_visible: bool,
    pub(crate) alert_label_input: String,

    /// Order entry form.
    pub(crate) order_symbol: String,
    pub(crate) order_qty: String,
    pub(crate) order_side: usize, // 0=buy, 1=sell

    // ── Fear & Greed Index ───────────────────────────────────────────────
    pub(crate) show_fear_greed: bool,
    pub(crate) fear_greed_value: u32,    // 0-100
    pub(crate) fear_greed_label: String, // "Extreme Fear", "Fear", "Neutral", "Greed", "Extreme Greed"

    // ── World Indices Dashboard ─────────────────────────────────────────
    pub(crate) show_world_indices: bool,
    pub(crate) world_indices_data: Vec<WatchlistRow>,

    // ── Crypto Top 50 ───────────────────────────────────────────────────
    pub(crate) show_crypto_top50: bool,
    pub(crate) crypto_top50: Vec<(String, f64, f64, f64)>, // (name, price, change_24h%, market_cap)

    // ── Forex Major Pairs ───────────────────────────────────────────────
    pub(crate) show_forex_matrix: bool,
    pub(crate) forex_pairs_data: Vec<WatchlistRow>,

    // Company events, sentiment, transcripts, commodities, and tape research
    /// DES command — comprehensive company overview.
    pub(crate) show_company_desc: bool,
    pub(crate) desc_symbol: String,
    pub(crate) desc_profile: Option<typhoon_engine::core::research::CompanyProfile>,
    pub(crate) desc_peers: Vec<String>,
    pub(crate) desc_earnings: Vec<typhoon_engine::core::research::EarningRow>,
    pub(crate) desc_press: Vec<typhoon_engine::core::research::PressRelease>,
    pub(crate) desc_loading: bool,

    /// IPO command — upcoming IPO calendar.
    pub(crate) show_ipo_calendar: bool,
    pub(crate) ipo_events: Vec<typhoon_engine::core::research::IpoEvent>,
    pub(crate) ipo_loading: bool,
    pub(crate) ipo_sort_col: usize,
    pub(crate) ipo_sort_asc: bool,

    /// EARNINGS command — historical actuals vs estimates.
    pub(crate) show_earnings_history: bool,
    pub(crate) earnings_history_symbol: String,
    pub(crate) earnings_history_rows: Vec<typhoon_engine::core::research::EarningRow>,
    pub(crate) earnings_history_loading: bool,
    pub(crate) earnings_history_sort_col: usize,
    pub(crate) earnings_history_sort_asc: bool,

    /// PEERS command — related tickers.
    pub(crate) show_peers: bool,
    pub(crate) peers_symbol: String,
    pub(crate) peers_list: Vec<String>,
    pub(crate) peers_loading: bool,

    /// PRESS command — company press releases.
    pub(crate) show_press_releases: bool,
    pub(crate) press_symbol: String,
    pub(crate) press_releases_list: Vec<typhoon_engine::core::research::PressRelease>,
    pub(crate) press_loading: bool,

    /// SENTIMENT command — Reddit + Twitter social sentiment.
    pub(crate) show_sentiment: bool,
    pub(crate) sentiment_symbol: String,
    pub(crate) sentiment_rows: Vec<typhoon_engine::core::research::SocialSentimentRow>,
    pub(crate) sentiment_loading: bool,
    pub(crate) sentiment_sort_col: usize,
    pub(crate) sentiment_sort_asc: bool,

    /// TRANSCRIPTS command — earnings call transcripts.
    pub(crate) show_transcripts: bool,
    pub(crate) transcripts_symbol: String,
    pub(crate) transcripts_list: Vec<typhoon_engine::core::research::TranscriptMeta>,
    pub(crate) transcripts_selected: Option<usize>,
    pub(crate) transcripts_body: Option<typhoon_engine::core::research::Transcript>,
    pub(crate) transcripts_loading_list: bool,
    pub(crate) transcripts_loading_body: bool,
    #[allow(dead_code)]
    pub(crate) transcripts_summary: Option<typhoon_engine::core::sec_filing::FilingSummary>,
    #[allow(dead_code)]
    pub(crate) transcripts_summary_for: (String, i32, i32),

    /// GLCO command — global commodities futures dashboard.
    pub(crate) show_commodities: bool,
    pub(crate) commodities_quotes: Vec<typhoon_engine::core::research::CommodityQuote>,
    pub(crate) commodities_last_fetch: Option<std::time::Instant>,
    pub(crate) commodities_loading: bool,

    /// TAS command — live Time & Sales tape for the active chart symbol.
    /// (symbol, price, size, side, timestamp) — most recent at front.
    pub(crate) show_tas: bool,
    pub(crate) tas_symbol: String,
    pub(crate) tas_rows: VecDeque<(String, f64, f64, String, String)>,
    pub(crate) tas_paused: bool,

    // Dividend, earnings-estimate, rating, and treasury research
    /// DVD — per-symbol dividend history.
    pub(crate) show_dividend_history: bool,
    pub(crate) dividend_history_symbol: String,
    pub(crate) dividend_history: Vec<typhoon_engine::core::research::DividendRecord>,
    pub(crate) dividend_history_loading: bool,

    /// EEB — forward earnings estimates.
    pub(crate) show_earnings_estimates: bool,
    pub(crate) earnings_estimates_symbol: String,
    pub(crate) earnings_estimates: Vec<typhoon_engine::core::research::EarningsEstimate>,
    pub(crate) earnings_estimates_loading: bool,

    /// UPDG — analyst rating change feed (upgrades/downgrades).
    pub(crate) show_rating_changes: bool,
    pub(crate) rating_changes_symbol: String,
    pub(crate) rating_changes: Vec<typhoon_engine::core::research::RatingChange>,
    pub(crate) rating_changes_loading: bool,

    /// GY — US Treasury yield curve snapshot.
    pub(crate) show_treasury_curve: bool,
    pub(crate) treasury_yields: Vec<typhoon_engine::core::research::TreasuryYield>,
    pub(crate) treasury_yields_last_fetch: Option<std::time::Instant>,
    pub(crate) treasury_yields_loading: bool,

    // Financial statements, management, and COT research
    /// FA — full financial statements bundle (Income / Balance / Cash Flow).
    pub(crate) show_financials: bool,
    pub(crate) financials_symbol: String,
    pub(crate) financials: typhoon_engine::core::research::FinancialStatements,
    pub(crate) financials_loading: bool,
    pub(crate) financials_view: FinancialsView,
    pub(crate) financials_period: FinancialsPeriod,

    /// MGMT — company officers + compensation.
    pub(crate) show_executives: bool,
    pub(crate) executives_symbol: String,
    pub(crate) executives: Vec<typhoon_engine::core::research::Executive>,
    pub(crate) executives_loading: bool,

    /// COT — CFTC Commitments of Traders (weekly, global).
    pub(crate) show_cot: bool,
    pub(crate) cot_reports: Vec<typhoon_engine::core::research::CotReport>,
    pub(crate) cot_loading: bool,
    pub(crate) cot_last_fetch: Option<std::time::Instant>,
    pub(crate) cot_filter: String,

    // Corporate action, analyst, ESG, ETF, and index research
    /// SPLT — historical stock split events.
    pub(crate) show_splits: bool,
    pub(crate) splits_symbol: String,
    pub(crate) splits_list: Vec<typhoon_engine::core::research::StockSplit>,
    pub(crate) splits_loading: bool,

    /// ETF — exchange-traded fund holdings (constituents).
    pub(crate) show_etf_holdings: bool,
    pub(crate) etf_symbol: String,
    pub(crate) etf_holdings: Vec<typhoon_engine::core::research::EtfHolding>,
    pub(crate) etf_loading: bool,

    /// ANR — analyst recommendation buckets + consensus price target.
    pub(crate) show_analyst_recs: bool,
    pub(crate) anr_symbol: String,
    pub(crate) analyst_recs: Vec<typhoon_engine::core::research::AnalystRecommendation>,
    pub(crate) price_target: typhoon_engine::core::research::PriceTarget,
    pub(crate) anr_loading: bool,

    /// ESG — environmental / social / governance scores by year.
    pub(crate) show_esg: bool,
    pub(crate) esg_symbol: String,
    pub(crate) esg_rows: Vec<typhoon_engine::core::research::EsgScore>,
    pub(crate) esg_loading: bool,

    /// MEMB — equity index constituents (global, cached by index code).
    pub(crate) show_index_members: bool,
    pub(crate) index_code: String,
    pub(crate) index_members: Vec<typhoon_engine::core::research::IndexMember>,
    pub(crate) memb_loading: bool,
    pub(crate) memb_filter: String,

    // Ownership, float, price-history, and earnings-surprise research
    /// INS — SEC Form-4 insider trades.
    pub(crate) show_insider_trades: bool,
    pub(crate) insider_symbol: String,
    pub(crate) insider_trades: Vec<typhoon_engine::core::research::InsiderTrade>,
    pub(crate) insider_loading: bool,

    /// HDS — 13F-derived institutional holders.
    pub(crate) show_inst_holders: bool,
    pub(crate) inst_holders_symbol: String,
    pub(crate) institutional_holders: Vec<typhoon_engine::core::research::InstitutionalHolder>,
    pub(crate) inst_holders_loading: bool,

    /// FLOAT — shares float + outstanding snapshot.
    pub(crate) show_shares_float: bool,
    pub(crate) float_symbol: String,
    pub(crate) shares_float: typhoon_engine::core::research::SharesFloat,
    pub(crate) float_loading: bool,

    /// HP — historical price table (daily OHLCV).
    pub(crate) show_hist_price: bool,
    pub(crate) hp_symbol: String,
    pub(crate) hp_rows: Vec<typhoon_engine::core::research::HistoricalPriceRow>,
    pub(crate) hp_loading: bool,
    pub(crate) hp_limit: usize,

    /// EPS — quarterly earnings surprise history.
    pub(crate) show_eps_surprise: bool,
    pub(crate) eps_symbol: String,
    pub(crate) eps_surprises: Vec<typhoon_engine::core::research::EarningsSurprise>,
    pub(crate) eps_loading: bool,

    // World index, market mover, sector, and WACC research
    /// WEI — world equity indices dashboard (Yahoo index tickers, separate
    /// from the legacy ETF-based "World Indices" dashboard above).
    pub(crate) show_wei: bool,
    pub(crate) wei_indices: Vec<typhoon_engine::core::research::WorldIndex>,
    pub(crate) wei_loading: bool,
    pub(crate) wei_region_filter: String, // "" | "Americas" | "EMEA" | "Asia-Pacific"

    /// MOV — market movers (gainers / losers / actives).
    pub(crate) show_market_movers: bool,
    pub(crate) market_movers: typhoon_engine::core::research::MarketMovers,
    pub(crate) mov_loading: bool,

    /// INDU — sector performance snapshot.
    pub(crate) show_sector_perf: bool,
    pub(crate) sector_perf: Vec<typhoon_engine::core::research::SectorPerformance>,
    pub(crate) indu_loading: bool,

    /// CACS — corporate-actions calendar aggregator (UI-only, reuses cached
    /// splits / dividends / earnings / IPO data).
    pub(crate) show_cacs: bool,
    pub(crate) cacs_symbol: String,

    /// WACC — derived cost-of-capital snapshot (per symbol).
    pub(crate) show_wacc: bool,
    pub(crate) wacc_symbol: String,
    pub(crate) wacc_snapshot: typhoon_engine::core::research::WaccSnapshot,
    pub(crate) wacc_loading: bool,

    // FX, beta, valuation, and identifier research
    /// WCR — world currency rates (FX majors + crosses + EM), Yahoo-sourced
    /// single-row snapshot. Separate state from the legacy FOREX_MATRIX
    /// dashboard which is broker-sourced.
    pub(crate) show_wcr: bool,
    pub(crate) wcr_rates: Vec<typhoon_engine::core::research::CurrencyRate>,
    pub(crate) wcr_loading: bool,
    pub(crate) wcr_region_filter: String, // "" | "Majors" | "Crosses" | "EM"

    /// BETA — rolling beta history vs SPY (1Y/3Y/5Y windows).
    pub(crate) show_beta: bool,
    pub(crate) beta_symbol: String,
    pub(crate) beta_snapshot: typhoon_engine::core::research::BetaSnapshot,
    pub(crate) beta_loading: bool,

    /// DDM — Gordon Growth dividend-discount-model snapshot.
    pub(crate) show_ddm: bool,
    pub(crate) ddm_symbol: String,
    pub(crate) ddm_snapshot: typhoon_engine::core::research::DdmSnapshot,
    pub(crate) ddm_loading: bool,

    /// RV — relative valuation peer matrix (zero-fetch, pure compute).
    pub(crate) show_rv: bool,
    pub(crate) rv_symbol: String,
    pub(crate) rv_snapshot: typhoon_engine::core::research::RelativeValuation,
    pub(crate) rv_loading: bool,

    /// FIGI — OpenFIGI identifier mapping.
    pub(crate) show_figi: bool,
    pub(crate) figi_symbol: String,
    pub(crate) figi_snapshot: typhoon_engine::core::research::FigiSnapshot,
    pub(crate) figi_loading: bool,

    // Advanced valuation, options, and implied-volatility research
    /// HRA — historical return / risk analysis (vol, Sharpe, Sortino, drawdowns).
    pub(crate) show_hra: bool,
    pub(crate) hra_symbol: String,
    pub(crate) hra_snapshot: typhoon_engine::core::research::HraSnapshot,
    pub(crate) hra_loading: bool,

    /// DCF — discounted cash flow fair value (FCFF model).
    pub(crate) show_dcf: bool,
    pub(crate) dcf_symbol: String,
    pub(crate) dcf_snapshot: typhoon_engine::core::research::DcfSnapshot,
    pub(crate) dcf_growth_pct: f64,
    pub(crate) dcf_terminal_growth_pct: f64,
    pub(crate) dcf_projection_years: usize,
    pub(crate) dcf_loading: bool,

    /// SVM — stock valuation model synthesis (DDM + DCF + peer multiples).
    pub(crate) show_svm: bool,
    pub(crate) svm_symbol: String,
    pub(crate) svm_snapshot: typhoon_engine::core::research::SvmSnapshot,
    pub(crate) svm_loading: bool,

    /// OMON — Yahoo options chain monitor.
    pub(crate) show_omon: bool,
    pub(crate) omon_symbol: String,
    pub(crate) omon_snapshot: typhoon_engine::core::research::OptionsChainSnapshot,
    pub(crate) omon_loading: bool,

    /// IVOL — implied-vol rank / percentile from cached OMON history.
    pub(crate) show_ivol: bool,
    pub(crate) ivol_symbol: String,
    pub(crate) ivol_snapshot: typhoon_engine::core::research::IvolSnapshot,
    pub(crate) ivol_loading: bool,

    // Seasonality, correlation, total-return, technical, and vol-skew research
    /// SEAG — monthly + day-of-week seasonality over cached HP.
    pub(crate) show_seag: bool,
    pub(crate) seag_symbol: String,
    pub(crate) seag_snapshot: typhoon_engine::core::research::SeasonalitySnapshot,
    pub(crate) seag_loading: bool,

    /// COR — correlation matrix vs peers.
    pub(crate) show_cor: bool,
    pub(crate) cor_symbol: String,
    pub(crate) cor_snapshot: typhoon_engine::core::research::CorrelationMatrix,
    pub(crate) cor_window_days: usize,
    pub(crate) cor_loading: bool,

    /// TRA — total return (price + dividends) snapshot.
    pub(crate) show_tra: bool,
    pub(crate) tra_symbol: String,
    pub(crate) tra_snapshot: typhoon_engine::core::research::TotalReturnSnapshot,
    pub(crate) tra_loading: bool,

    /// TECH — technical-indicator dashboard.
    pub(crate) show_tech: bool,
    pub(crate) tech_symbol: String,
    pub(crate) tech_snapshot: typhoon_engine::core::research::TechnicalSnapshot,
    pub(crate) tech_loading: bool,

    /// SKEW — volatility-skew/smile over cached OMON.
    pub(crate) show_skew: bool,
    pub(crate) skew_symbol: String,
    pub(crate) skew_snapshot: typhoon_engine::core::research::VolatilitySkew,
    pub(crate) skew_loading: bool,

    // Leverage, accruals, realized-volatility, cash-flow, and short-interest research
    /// LEV — debt leverage & coverage ratios from cached Financials + Fundamentals.
    pub(crate) show_lev: bool,
    pub(crate) lev_symbol: String,
    pub(crate) lev_snapshot: typhoon_engine::core::research::LeverageSnapshot,
    pub(crate) lev_loading: bool,

    /// ACRL — earnings quality (NI vs FCF) from cached quarterly Financials.
    pub(crate) show_acrl: bool,
    pub(crate) acrl_symbol: String,
    pub(crate) acrl_snapshot: typhoon_engine::core::research::AccrualsSnapshot,
    pub(crate) acrl_loading: bool,

    /// RVOL — realized volatility cone from cached HP bars.
    pub(crate) show_rvol: bool,
    pub(crate) rvol_symbol: String,
    pub(crate) rvol_snapshot: typhoon_engine::core::research::RealizedVolSnapshot,
    pub(crate) rvol_loading: bool,

    /// FCFY — FCF yield, payout ratios, dividend sustainability from cached Financials.
    pub(crate) show_fcfy: bool,
    pub(crate) fcfy_symbol: String,
    pub(crate) fcfy_snapshot: typhoon_engine::core::research::FcfYieldSnapshot,
    pub(crate) fcfy_loading: bool,

    /// SHRT — short interest / days-to-cover from cached SharesFloat + HP bars.
    pub(crate) show_shrt: bool,
    pub(crate) shrt_symbol: String,
    pub(crate) shrt_snapshot: typhoon_engine::core::research::ShortInterestSnapshot,
    pub(crate) shrt_loading: bool,

    // Solvency, quality, volatility-estimator, EPS-beat, and price-target research
    /// ALTZ — classic Altman Z-score from cached Financials + Fundamentals.
    pub(crate) show_altz: bool,
    pub(crate) altz_symbol: String,
    pub(crate) altz_snapshot: typhoon_engine::core::research::AltmanZSnapshot,
    pub(crate) altz_loading: bool,

    /// PTFS — Piotroski F-score over 2 annual periods from cached Financials.
    pub(crate) show_ptfs: bool,
    pub(crate) ptfs_symbol: String,
    pub(crate) ptfs_snapshot: typhoon_engine::core::research::PiotroskiSnapshot,
    pub(crate) ptfs_loading: bool,

    /// VOLE — OHLC volatility estimators (CtC / Parkinson / GK / RS / YZ) from cached HP bars.
    pub(crate) show_vole: bool,
    pub(crate) vole_symbol: String,
    pub(crate) vole_snapshot: typhoon_engine::core::research::OhlcVolSnapshot,
    pub(crate) vole_loading: bool,

    /// EPSB — EPS beat streak & surprise analysis from cached earnings surprise history.
    pub(crate) show_epsb: bool,
    pub(crate) epsb_symbol: String,
    pub(crate) epsb_snapshot: typhoon_engine::core::research::EpsBeatSnapshot,
    pub(crate) epsb_loading: bool,

    /// PTD — Price target dispersion & implied return from cached PriceTarget + current price.
    pub(crate) show_ptd: bool,
    pub(crate) ptd_symbol: String,
    pub(crate) ptd_snapshot: typhoon_engine::core::research::PriceTargetDispersion,
    pub(crate) ptd_loading: bool,

    // Insider, dividend-growth, earnings-revision, sector-rotation, and upgrade/downgrade research
    /// MNGR — Insider activity bias over cached INS form-4 trades in a lookback window.
    pub(crate) show_mngr: bool,
    pub(crate) mngr_symbol: String,
    pub(crate) mngr_window_days: i32,
    pub(crate) mngr_snapshot: typhoon_engine::core::research::InsiderActivitySnapshot,
    pub(crate) mngr_loading: bool,

    /// DIVG — Dividend growth analysis (CAGR, consistency) from cached DVD history.
    pub(crate) show_divg: bool,
    pub(crate) divg_symbol: String,
    pub(crate) divg_snapshot: typhoon_engine::core::research::DivgSnapshot,
    pub(crate) divg_loading: bool,

    /// EARM — Earnings momentum trend from cached FA + EPS surprises.
    pub(crate) show_earm: bool,
    pub(crate) earm_symbol: String,
    pub(crate) earm_snapshot: typhoon_engine::core::research::EarmSnapshot,
    pub(crate) earm_loading: bool,

    /// SECTR — Sector rotation strength from cached INDU + Fundamentals.sector.
    pub(crate) show_sectr: bool,
    pub(crate) sectr_symbol: String,
    pub(crate) sectr_snapshot: typhoon_engine::core::research::SectorRotationSnapshot,
    pub(crate) sectr_loading: bool,

    /// UPDM — Upgrade/downgrade momentum from cached UPDG rating changes.
    pub(crate) show_updm: bool,
    pub(crate) updm_symbol: String,
    pub(crate) updm_snapshot: typhoon_engine::core::research::UpdmSnapshot,
    pub(crate) updm_loading: bool,

    // Momentum, liquidity, breakout, cash-cycle, and credit research
    /// MOM — 12-1 month momentum score from cached HP bars.
    pub(crate) show_mom: bool,
    pub(crate) mom_symbol: String,
    pub(crate) mom_snapshot: typhoon_engine::core::research::MomentumSnapshot,
    pub(crate) mom_loading: bool,

    /// LIQ — Liquidity profile from cached HP bars + Fundamentals.
    pub(crate) show_liq: bool,
    pub(crate) liq_symbol: String,
    pub(crate) liq_window_days: i32,
    pub(crate) liq_snapshot: typhoon_engine::core::research::LiquiditySnapshot,
    pub(crate) liq_loading: bool,

    /// BREAK — Breakout proximity from cached HP bars.
    pub(crate) show_break: bool,
    pub(crate) break_symbol: String,
    pub(crate) break_snapshot: typhoon_engine::core::research::BreakoutSnapshot,
    pub(crate) break_loading: bool,

    /// CCRL — Cash conversion cycle from cached FA statements.
    pub(crate) show_ccrl: bool,
    pub(crate) ccrl_symbol: String,
    pub(crate) ccrl_snapshot: typhoon_engine::core::research::CashCycleSnapshot,
    pub(crate) ccrl_loading: bool,

    /// CREDIT — Unified credit score fusing cached ALTZ + PTFS + LEV + ACRL.
    pub(crate) show_credit: bool,
    pub(crate) credit_symbol: String,
    pub(crate) credit_snapshot: typhoon_engine::core::research::CreditSnapshot,
    pub(crate) credit_loading: bool,

    // Growth, flow, regime, relative-volume, and margin research
    /// GROWM — GARP composite fusing cached MOM + EARM + DIVG.
    pub(crate) show_growm: bool,
    pub(crate) growm_symbol: String,
    pub(crate) growm_snapshot: typhoon_engine::core::research::GrowmSnapshot,
    pub(crate) growm_loading: bool,

    /// FLOW — Insider + institutional flow score.
    pub(crate) show_flow: bool,
    pub(crate) flow_symbol: String,
    pub(crate) flow_window_days: i32,
    pub(crate) flow_snapshot: typhoon_engine::core::research::FlowSnapshot,
    pub(crate) flow_loading: bool,

    /// REGIME — Market regime classifier fusing VOLE + TECH + HRA.
    pub(crate) show_regime: bool,
    pub(crate) regime_symbol: String,
    pub(crate) regime_snapshot: typhoon_engine::core::research::RegimeSnapshot,
    pub(crate) regime_loading: bool,

    /// RELVOL — Relative volume vs trailing averages.
    pub(crate) show_relvol: bool,
    pub(crate) relvol_symbol: String,
    pub(crate) relvol_snapshot: typhoon_engine::core::research::RelVolSnapshot,
    pub(crate) relvol_loading: bool,

    /// MARGINS — Margin trajectory (gross/op/net) over cached FA statements.
    pub(crate) show_margins: bool,
    pub(crate) margins_symbol: String,
    pub(crate) margins_snapshot: typhoon_engine::core::research::MarginsSnapshot,
    pub(crate) margins_loading: bool,

    // Value, quality, risk, insider-streak, and coverage research
    /// VAL — Value-factor composite vs sector peers.
    pub(crate) show_val: bool,
    pub(crate) val_symbol: String,
    pub(crate) val_snapshot: typhoon_engine::core::research::ValueSnapshot,
    pub(crate) val_loading: bool,

    /// QUAL — Quality-factor composite fusing PTFS + MARGINS + ACRL + LEV.
    pub(crate) show_qual: bool,
    pub(crate) qual_symbol: String,
    pub(crate) qual_snapshot: typhoon_engine::core::research::QualitySnapshot,
    pub(crate) qual_loading: bool,

    /// RISK — Risk-factor composite fusing VOLE + BETA + LIQ + SHRT + ALTZ.
    pub(crate) show_risk: bool,
    pub(crate) risk_symbol: String,
    pub(crate) risk_snapshot: typhoon_engine::core::research::RiskSnapshot,
    pub(crate) risk_loading: bool,

    /// INSSTRK — Insider streak detector from cached Form 4 trades.
    pub(crate) show_insstrk: bool,
    pub(crate) insstrk_symbol: String,
    pub(crate) insstrk_window_days: i32,
    pub(crate) insstrk_snapshot: typhoon_engine::core::research::InsiderStreakSnapshot,
    pub(crate) insstrk_loading: bool,

    /// COVG — Analyst coverage breadth + churn snapshot.
    pub(crate) show_covg: bool,
    pub(crate) covg_symbol: String,
    pub(crate) covg_snapshot: typhoon_engine::core::research::CoverageSnapshot,
    pub(crate) covg_loading: bool,

    // Relative rank and event-study research
    /// VRK — Value Rank vs sector peers.
    pub(crate) show_vrk: bool,
    pub(crate) vrk_symbol: String,
    pub(crate) vrk_snapshot: typhoon_engine::core::research::ValueRankSnapshot,
    pub(crate) vrk_loading: bool,

    /// QRK — Quality Rank vs sector peers.
    pub(crate) show_qrk: bool,
    pub(crate) qrk_symbol: String,
    pub(crate) qrk_snapshot: typhoon_engine::core::research::QualityRankSnapshot,
    pub(crate) qrk_loading: bool,

    /// RRK — Risk Rank vs sector peers (inverted — higher rank = safer).
    pub(crate) show_rrk: bool,
    pub(crate) rrk_symbol: String,
    pub(crate) rrk_snapshot: typhoon_engine::core::research::RiskRankSnapshot,
    pub(crate) rrk_loading: bool,

    /// RELEPSGR — Relative 3y EPS CAGR vs sector median.
    pub(crate) show_relepsgr: bool,
    pub(crate) relepsgr_symbol: String,
    pub(crate) relepsgr_snapshot: typhoon_engine::core::research::RelativeEpsGrowthSnapshot,
    pub(crate) relepsgr_loading: bool,

    /// PEAD — Post-Earnings-Announcement Drift.
    pub(crate) show_pead: bool,
    pub(crate) pead_symbol: String,
    pub(crate) pead_snapshot: typhoon_engine::core::research::PeadSnapshot,
    pub(crate) pead_loading: bool,

    // Size, momentum, drift, operating-quality, and revenue-growth ranks
    /// SIZEF — Size factor rank vs sector peers.
    pub(crate) show_sizef: bool,
    pub(crate) sizef_symbol: String,
    pub(crate) sizef_snapshot: typhoon_engine::core::research::SizeFactorSnapshot,
    pub(crate) sizef_loading: bool,

    /// MOMF — Momentum factor rank vs sector peers.
    pub(crate) show_momf: bool,
    pub(crate) momf_symbol: String,
    pub(crate) momf_snapshot: typhoon_engine::core::research::MomentumRankSnapshot,
    pub(crate) momf_loading: bool,

    /// PEADRANK — PEAD drift rank vs sector peers.
    pub(crate) show_peadrank: bool,
    pub(crate) peadrank_symbol: String,
    pub(crate) peadrank_snapshot: typhoon_engine::core::research::PeadRankSnapshot,
    pub(crate) peadrank_loading: bool,

    /// FQM — Fundamental Quality Meter (Piotroski + margins + accruals).
    pub(crate) show_fqm: bool,
    pub(crate) fqm_symbol: String,
    pub(crate) fqm_snapshot: typhoon_engine::core::research::FundamentalQualityMeterSnapshot,
    pub(crate) fqm_loading: bool,

    /// REVRANK — Relative 3y revenue CAGR vs sector median.
    pub(crate) show_revrank: bool,
    pub(crate) revrank_symbol: String,
    pub(crate) revrank_snapshot: typhoon_engine::core::research::RevenueGrowthRankSnapshot,
    pub(crate) revrank_loading: bool,

    // Financial growth, rank overlay, and surprise-streak research
    /// LEVRANK — Leverage rank vs sector peers (D/E percentile, inverted).
    pub(crate) show_levrank: bool,
    pub(crate) levrank_symbol: String,
    pub(crate) levrank_snapshot: typhoon_engine::core::research::LeverageRankSnapshot,
    pub(crate) levrank_loading: bool,

    /// OPERANK — Operating Quality rank vs sector peers.
    pub(crate) show_operank: bool,
    pub(crate) operank_symbol: String,
    pub(crate) operank_snapshot: typhoon_engine::core::research::OperatingQualityRankSnapshot,
    pub(crate) operank_loading: bool,

    /// FQMRANK — Fundamental Quality Meter rank vs sector peers.
    pub(crate) show_fqmrank: bool,
    pub(crate) fqmrank_symbol: String,
    pub(crate) fqmrank_snapshot: typhoon_engine::core::research::FqmRankSnapshot,
    pub(crate) fqmrank_loading: bool,

    /// LIQRANK — Liquidity rank vs sector peers.
    pub(crate) show_liqrank: bool,
    pub(crate) liqrank_symbol: String,
    pub(crate) liqrank_snapshot: typhoon_engine::core::research::LiquidityRankSnapshot,
    pub(crate) liqrank_loading: bool,

    /// SURPSTK — Earnings surprise streak stat.
    pub(crate) show_surpstk: bool,
    pub(crate) surpstk_symbol: String,
    pub(crate) surpstk_snapshot: typhoon_engine::core::research::EarningsSurpriseStreakSnapshot,
    pub(crate) surpstk_loading: bool,

    /// DVDRANK — Dividend growth rank vs sector peers.
    pub(crate) show_dvdrank: bool,
    pub(crate) dvdrank_symbol: String,
    pub(crate) dvdrank_snapshot: typhoon_engine::core::research::DividendGrowthRankSnapshot,
    pub(crate) dvdrank_loading: bool,

    /// EARMRANK — Earnings momentum rank vs sector peers.
    pub(crate) show_earmrank: bool,
    pub(crate) earmrank_symbol: String,
    pub(crate) earmrank_snapshot: typhoon_engine::core::research::EarningsMomentumRankSnapshot,
    pub(crate) earmrank_loading: bool,

    /// UPDGRANK — Upgrade/downgrade rank vs sector peers.
    pub(crate) show_updgrank: bool,
    pub(crate) updgrank_symbol: String,
    pub(crate) updgrank_snapshot: typhoon_engine::core::research::UpgradeDowngradeRankSnapshot,
    pub(crate) updgrank_loading: bool,

    /// GY — Gap yearly stat.
    pub(crate) show_gy: bool,
    pub(crate) gy_symbol: String,
    pub(crate) gy_snapshot: typhoon_engine::core::research::GapYearlySnapshot,
    pub(crate) gy_loading: bool,

    /// DES — Daily event streak stat.
    pub(crate) show_des: bool,
    pub(crate) des_symbol: String,
    pub(crate) des_snapshot: typhoon_engine::core::research::DailyEventStreakSnapshot,
    pub(crate) des_loading: bool,

    /// DVDYIELDRANK — Dividend yield rank vs sector peers.
    pub(crate) show_dvdyieldrank: bool,
    pub(crate) dvdyieldrank_symbol: String,
    pub(crate) dvdyieldrank_snapshot: typhoon_engine::core::research::DividendYieldRankSnapshot,
    pub(crate) dvdyieldrank_loading: bool,

    /// SHRANK — Short interest rank vs sector peers (risk-inverted).
    pub(crate) show_shrank: bool,
    pub(crate) shrank_symbol: String,
    pub(crate) shrank_snapshot: typhoon_engine::core::research::ShortInterestRankSnapshot,
    pub(crate) shrank_loading: bool,

    /// SHORTRANK_DELTA — short-interest trend rank vs sector peers.
    pub(crate) show_shortrank_delta: bool,
    pub(crate) shortrank_delta_symbol: String,
    pub(crate) shortrank_delta_snapshot:
        typhoon_engine::core::research::ShortInterestDeltaRankSnapshot,
    pub(crate) shortrank_delta_loading: bool,

    /// INSIDERCONC — insider ownership concentration vs sector peers.
    pub(crate) show_insiderconc: bool,
    pub(crate) insiderconc_symbol: String,
    pub(crate) insiderconc_snapshot: typhoon_engine::core::research::InsiderConcentrationSnapshot,
    pub(crate) insiderconc_loading: bool,

    /// ATRANN — Annualized ATR volatility regime.
    pub(crate) show_atrann: bool,
    pub(crate) atrann_symbol: String,
    pub(crate) atrann_snapshot: typhoon_engine::core::research::AnnualizedAtrSnapshot,
    pub(crate) atrann_loading: bool,

    /// DDHIST — Drawdown history.
    pub(crate) show_ddhist: bool,
    pub(crate) ddhist_symbol: String,
    pub(crate) ddhist_snapshot: typhoon_engine::core::research::DrawdownHistorySnapshot,
    pub(crate) ddhist_loading: bool,

    /// PRICEPERF — Multi-horizon price performance.
    pub(crate) show_priceperf: bool,
    pub(crate) priceperf_symbol: String,
    pub(crate) priceperf_snapshot: typhoon_engine::core::research::PricePerformanceSnapshot,
    pub(crate) priceperf_loading: bool,

    /// MOMRANK_MULTI — sector-relative rank of PRICEPERF horizons.
    pub(crate) show_momrank_multi: bool,
    pub(crate) momrank_multi_symbol: String,
    pub(crate) momrank_multi_snapshot: typhoon_engine::core::research::MomentumRankMultiSnapshot,
    pub(crate) momrank_multi_loading: bool,

    /// BETARANK — Beta rank vs sector peers (risk-inverted).
    pub(crate) show_betarank: bool,
    pub(crate) betarank_symbol: String,
    pub(crate) betarank_snapshot: typhoon_engine::core::research::BetaRankSnapshot,
    pub(crate) betarank_loading: bool,

    /// PEGRANK — PEG ratio rank vs sector peers.
    pub(crate) show_pegrank: bool,
    pub(crate) pegrank_symbol: String,
    pub(crate) pegrank_snapshot: typhoon_engine::core::research::PegRankSnapshot,
    pub(crate) pegrank_loading: bool,

    /// FHIGHLOW — 52-week high/low distance + proximity band.
    pub(crate) show_fhighlow: bool,
    pub(crate) fhighlow_symbol: String,
    pub(crate) fhighlow_snapshot: typhoon_engine::core::research::FiftyTwoWeekHighLowSnapshot,
    pub(crate) fhighlow_loading: bool,

    /// RVCONE — Realized volatility cone (multi-horizon).
    pub(crate) show_rvcone: bool,
    pub(crate) rvcone_symbol: String,
    pub(crate) rvcone_snapshot: typhoon_engine::core::research::RealizedVolConeSnapshot,
    pub(crate) rvcone_loading: bool,

    /// CALPB — Calendar period breakdowns.
    pub(crate) show_calpb: bool,
    pub(crate) calpb_symbol: String,
    pub(crate) calpb_snapshot: typhoon_engine::core::research::CalendarPeriodBreakdownSnapshot,
    pub(crate) calpb_loading: bool,

    /// CORRSTK — rolling correlation vs SPY / sector ETF benchmark.
    pub(crate) show_corrstk: bool,
    pub(crate) corrstk_symbol: String,
    pub(crate) corrstk_snapshot: typhoon_engine::core::research::CorrStkSnapshot,
    pub(crate) corrstk_loading: bool,

    /// TLRANK — trailing 30d liquidity rank vs sector peers.
    pub(crate) show_tlrank: bool,
    pub(crate) tlrank_symbol: String,
    pub(crate) tlrank_snapshot: typhoon_engine::core::research::ThirtyDayLiquidityRankSnapshot,
    pub(crate) tlrank_loading: bool,

    /// CORRRANK — benchmark-linkage rank vs sector peers.
    pub(crate) show_corrrank: bool,
    pub(crate) corrrank_symbol: String,
    pub(crate) corrrank_snapshot: typhoon_engine::core::research::CorrelationRankSnapshot,
    pub(crate) corrrank_loading: bool,

    /// OPERANK_DELTA — operating-margin trend rank vs sector peers.
    pub(crate) show_operank_delta: bool,
    pub(crate) operank_delta_symbol: String,
    pub(crate) operank_delta_snapshot:
        typhoon_engine::core::research::OperatingMarginDeltaRankSnapshot,
    pub(crate) operank_delta_loading: bool,

    /// DIVACC — dividend growth acceleration.
    pub(crate) show_divacc: bool,
    pub(crate) divacc_symbol: String,
    pub(crate) divacc_snapshot: typhoon_engine::core::research::DividendAccelerationSnapshot,
    pub(crate) divacc_loading: bool,

    /// EPSACC — EPS acceleration from cached quarterly financials.
    pub(crate) show_epsacc: bool,
    pub(crate) epsacc_symbol: String,
    pub(crate) epsacc_snapshot: typhoon_engine::core::research::EpsAccelerationSnapshot,
    pub(crate) epsacc_loading: bool,

    /// VRP — implied-vs-realized vol premium.
    pub(crate) show_vrp: bool,
    pub(crate) vrp_symbol: String,
    pub(crate) vrp_snapshot: typhoon_engine::core::research::VolRiskPremiumSnapshot,
    pub(crate) vrp_loading: bool,

    /// RETSKEW — Return distribution skewness.
    pub(crate) show_retskew: bool,
    pub(crate) retskew_symbol: String,
    pub(crate) retskew_snapshot: typhoon_engine::core::research::ReturnSkewnessSnapshot,
    pub(crate) retskew_loading: bool,

    /// RETKURT — Return distribution excess kurtosis.
    pub(crate) show_retkurt: bool,
    pub(crate) retkurt_symbol: String,
    pub(crate) retkurt_snapshot: typhoon_engine::core::research::ReturnKurtosisSnapshot,
    pub(crate) retkurt_loading: bool,

    /// TAILR — Tail ratio.
    pub(crate) show_tailr: bool,
    pub(crate) tailr_symbol: String,
    pub(crate) tailr_snapshot: typhoon_engine::core::research::TailRatioSnapshot,
    pub(crate) tailr_loading: bool,

    /// RUNLEN — Up/down day run length stats.
    pub(crate) show_runlen: bool,
    pub(crate) runlen_symbol: String,
    pub(crate) runlen_snapshot: typhoon_engine::core::research::RunLengthSnapshot,
    pub(crate) runlen_loading: bool,

    /// DAYRANGE — Daily range analysis.
    pub(crate) show_dayrange: bool,
    pub(crate) dayrange_symbol: String,
    pub(crate) dayrange_snapshot: typhoon_engine::core::research::DailyRangeSnapshot,
    pub(crate) dayrange_loading: bool,

    // ── Research section ──
    /// AUTOCOR — Autocorrelation at multiple lags.
    pub(crate) show_autocor: bool,
    pub(crate) autocor_symbol: String,
    pub(crate) autocor_snapshot: typhoon_engine::core::research::AutocorrelationSnapshot,
    pub(crate) autocor_loading: bool,

    /// HURST — Hurst exponent via R/S analysis.
    pub(crate) show_hurst: bool,
    pub(crate) hurst_symbol: String,
    pub(crate) hurst_snapshot: typhoon_engine::core::research::HurstSnapshot,
    pub(crate) hurst_loading: bool,

    /// HITRATE — Multi-horizon hit rate.
    pub(crate) show_hitrate: bool,
    pub(crate) hitrate_symbol: String,
    pub(crate) hitrate_snapshot: typhoon_engine::core::research::HitRateSnapshot,
    pub(crate) hitrate_loading: bool,

    /// GLASYM — Gain/loss asymmetry.
    pub(crate) show_glasym: bool,
    pub(crate) glasym_symbol: String,
    pub(crate) glasym_snapshot: typhoon_engine::core::research::GainLossAsymmetrySnapshot,
    pub(crate) glasym_loading: bool,

    /// VOLRATIO — Up vs down volume ratio.
    pub(crate) show_volratio: bool,
    pub(crate) volratio_symbol: String,
    pub(crate) volratio_snapshot: typhoon_engine::core::research::VolumeRatioSnapshot,
    pub(crate) volratio_loading: bool,

    // ── Research section ──
    /// DRAWUP — Upside rally history (mirror of DDHIST).
    pub(crate) show_drawup: bool,
    pub(crate) drawup_symbol: String,
    pub(crate) drawup_snapshot: typhoon_engine::core::research::DrawupHistorySnapshot,
    pub(crate) drawup_loading: bool,

    /// GAPSTATS — Overnight gap statistics.
    pub(crate) show_gapstats: bool,
    pub(crate) gapstats_symbol: String,
    pub(crate) gapstats_snapshot: typhoon_engine::core::research::GapStatsSnapshot,
    pub(crate) gapstats_loading: bool,

    /// VOLCLUSTER — Volatility clustering autocorrelation.
    pub(crate) show_volcluster: bool,
    pub(crate) volcluster_symbol: String,
    pub(crate) volcluster_snapshot: typhoon_engine::core::research::VolClusterSnapshot,
    pub(crate) volcluster_loading: bool,

    /// CLOSEPLC — Close placement within daily range.
    pub(crate) show_closeplc: bool,
    pub(crate) closeplc_symbol: String,
    pub(crate) closeplc_snapshot: typhoon_engine::core::research::ClosePlacementSnapshot,
    pub(crate) closeplc_loading: bool,

    /// MRHL — AR(1) mean-reversion half-life.
    pub(crate) show_mrhl: bool,
    pub(crate) mrhl_symbol: String,
    pub(crate) mrhl_snapshot: typhoon_engine::core::research::MeanReversionHalfLifeSnapshot,
    pub(crate) mrhl_loading: bool,

    // ── Research section ──
    /// DOWNVOL — Downside deviation + Sortino ratio.
    pub(crate) show_downvol: bool,
    pub(crate) downvol_symbol: String,
    pub(crate) downvol_snapshot: typhoon_engine::core::research::DownsideVolSnapshot,
    pub(crate) downvol_loading: bool,

    /// SHARPR — Sharpe ratio snapshot.
    pub(crate) show_sharpr: bool,
    pub(crate) sharpr_symbol: String,
    pub(crate) sharpr_snapshot: typhoon_engine::core::research::SharpeRatioSnapshot,
    pub(crate) sharpr_loading: bool,

    /// EFFRATIO — Kaufman's efficiency ratio.
    pub(crate) show_effratio: bool,
    pub(crate) effratio_symbol: String,
    pub(crate) effratio_snapshot: typhoon_engine::core::research::EfficiencyRatioSnapshot,
    pub(crate) effratio_loading: bool,

    /// WICKBIAS — Upper vs lower wick asymmetry.
    pub(crate) show_wickbias: bool,
    pub(crate) wickbias_symbol: String,
    pub(crate) wickbias_snapshot: typhoon_engine::core::research::WickBiasSnapshot,
    pub(crate) wickbias_loading: bool,

    /// VOLOFVOL — Stdev of rolling 20d realized vol.
    pub(crate) show_volofvol: bool,
    pub(crate) volofvol_symbol: String,
    pub(crate) volofvol_snapshot: typhoon_engine::core::research::VolOfVolSnapshot,
    pub(crate) volofvol_loading: bool,

    // ── Research section ──
    pub(crate) show_calmar: bool,
    pub(crate) calmar_symbol: String,
    pub(crate) calmar_snapshot: typhoon_engine::core::research::CalmarRatioSnapshot,
    pub(crate) calmar_loading: bool,
    pub(crate) show_ulcer: bool,
    pub(crate) ulcer_symbol: String,
    pub(crate) ulcer_snapshot: typhoon_engine::core::research::UlcerIndexSnapshot,
    pub(crate) ulcer_loading: bool,
    pub(crate) show_varratio: bool,
    pub(crate) varratio_symbol: String,
    pub(crate) varratio_snapshot: typhoon_engine::core::research::VarianceRatioSnapshot,
    pub(crate) varratio_loading: bool,
    pub(crate) show_amihud: bool,
    pub(crate) amihud_symbol: String,
    pub(crate) amihud_snapshot: typhoon_engine::core::research::AmihudIlliqSnapshot,
    pub(crate) amihud_loading: bool,
    pub(crate) show_jbnorm: bool,
    pub(crate) jbnorm_symbol: String,
    pub(crate) jbnorm_snapshot: typhoon_engine::core::research::JarqueBeraSnapshot,
    pub(crate) jbnorm_loading: bool,

    // ── Research section ──
    pub(crate) show_omega: bool,
    pub(crate) omega_symbol: String,
    pub(crate) omega_snapshot: typhoon_engine::core::research::OmegaRatioSnapshot,
    pub(crate) omega_loading: bool,
    pub(crate) show_dfa: bool,
    pub(crate) dfa_symbol: String,
    pub(crate) dfa_snapshot: typhoon_engine::core::research::DetrendedFluctuationSnapshot,
    pub(crate) dfa_loading: bool,
    pub(crate) show_burke: bool,
    pub(crate) burke_symbol: String,
    pub(crate) burke_snapshot: typhoon_engine::core::research::BurkeRatioSnapshot,
    pub(crate) burke_loading: bool,
    pub(crate) show_monthseas: bool,
    pub(crate) monthseas_symbol: String,
    pub(crate) monthseas_snapshot: typhoon_engine::core::research::MonthlySeasonalitySnapshot,
    pub(crate) monthseas_loading: bool,
    pub(crate) show_rollsprd: bool,
    pub(crate) rollsprd_symbol: String,
    pub(crate) rollsprd_snapshot: typhoon_engine::core::research::RollSpreadSnapshot,
    pub(crate) rollsprd_loading: bool,

    // ── Research section ──
    pub(crate) show_parkinson: bool,
    pub(crate) parkinson_symbol: String,
    pub(crate) parkinson_snapshot: typhoon_engine::core::research::ParkinsonVolSnapshot,
    pub(crate) parkinson_loading: bool,
    pub(crate) show_gkvol: bool,
    pub(crate) gkvol_symbol: String,
    pub(crate) gkvol_snapshot: typhoon_engine::core::research::GarmanKlassVolSnapshot,
    pub(crate) gkvol_loading: bool,
    pub(crate) show_rsvol: bool,
    pub(crate) rsvol_symbol: String,
    pub(crate) rsvol_snapshot: typhoon_engine::core::research::RogersSatchellVolSnapshot,
    pub(crate) rsvol_loading: bool,
    pub(crate) show_cvar: bool,
    pub(crate) cvar_symbol: String,
    pub(crate) cvar_snapshot: typhoon_engine::core::research::CVaRSnapshot,
    pub(crate) cvar_loading: bool,
    pub(crate) show_doweffect: bool,
    pub(crate) doweffect_symbol: String,
    pub(crate) doweffect_snapshot: typhoon_engine::core::research::DayOfWeekEffectSnapshot,
    pub(crate) doweffect_loading: bool,

    // ── Research section ──
    pub(crate) show_sterling: bool,
    pub(crate) sterling_symbol: String,
    pub(crate) sterling_snapshot: typhoon_engine::core::research::SterlingRatioSnapshot,
    pub(crate) sterling_loading: bool,
    pub(crate) show_kellyf: bool,
    pub(crate) kellyf_symbol: String,
    pub(crate) kellyf_snapshot: typhoon_engine::core::research::KellyFractionSnapshot,
    pub(crate) kellyf_loading: bool,
    pub(crate) show_ljungb: bool,
    pub(crate) ljungb_symbol: String,
    pub(crate) ljungb_snapshot: typhoon_engine::core::research::LjungBoxSnapshot,
    pub(crate) ljungb_loading: bool,
    pub(crate) show_runstest: bool,
    pub(crate) runstest_symbol: String,
    pub(crate) runstest_snapshot: typhoon_engine::core::research::RunsTestSnapshot,
    pub(crate) runstest_loading: bool,
    pub(crate) show_zeroret: bool,
    pub(crate) zeroret_symbol: String,
    pub(crate) zeroret_snapshot: typhoon_engine::core::research::ZeroReturnSnapshot,
    pub(crate) zeroret_loading: bool,

    // ── Research section ──
    pub(crate) show_psr: bool,
    pub(crate) psr_symbol: String,
    pub(crate) psr_snapshot: typhoon_engine::core::research::ProbabilisticSharpeSnapshot,
    pub(crate) psr_loading: bool,
    pub(crate) show_adf: bool,
    pub(crate) adf_symbol: String,
    pub(crate) adf_snapshot: typhoon_engine::core::research::DickeyFullerSnapshot,
    pub(crate) adf_loading: bool,
    pub(crate) show_mnkendall: bool,
    pub(crate) mnkendall_symbol: String,
    pub(crate) mnkendall_snapshot: typhoon_engine::core::research::MannKendallSnapshot,
    pub(crate) mnkendall_loading: bool,
    pub(crate) show_bipower: bool,
    pub(crate) bipower_symbol: String,
    pub(crate) bipower_snapshot: typhoon_engine::core::research::BipowerVariationSnapshot,
    pub(crate) bipower_loading: bool,
    pub(crate) show_dddur: bool,
    pub(crate) dddur_symbol: String,
    pub(crate) dddur_snapshot: typhoon_engine::core::research::DrawdownDurationSnapshot,
    pub(crate) dddur_loading: bool,

    // ── Research section ──
    pub(crate) show_hilltail: bool,
    pub(crate) hilltail_symbol: String,
    pub(crate) hilltail_snapshot: typhoon_engine::core::research::HillTailSnapshot,
    pub(crate) hilltail_loading: bool,
    pub(crate) show_archlm: bool,
    pub(crate) archlm_symbol: String,
    pub(crate) archlm_snapshot: typhoon_engine::core::research::ArchLmSnapshot,
    pub(crate) archlm_loading: bool,
    pub(crate) show_painratio: bool,
    pub(crate) painratio_symbol: String,
    pub(crate) painratio_snapshot: typhoon_engine::core::research::PainRatioSnapshot,
    pub(crate) painratio_loading: bool,
    pub(crate) show_cusum: bool,
    pub(crate) cusum_symbol: String,
    pub(crate) cusum_snapshot: typhoon_engine::core::research::CusumBreakSnapshot,
    pub(crate) cusum_loading: bool,
    pub(crate) show_cfvar: bool,
    pub(crate) cfvar_symbol: String,
    pub(crate) cfvar_snapshot: typhoon_engine::core::research::CornishFisherSnapshot,
    pub(crate) cfvar_loading: bool,

    // ── Research section ──
    pub(crate) show_entropy: bool,
    pub(crate) entropy_symbol: String,
    pub(crate) entropy_snapshot: typhoon_engine::core::research::EntropySnapshot,
    pub(crate) entropy_loading: bool,
    pub(crate) show_rachev: bool,
    pub(crate) rachev_symbol: String,
    pub(crate) rachev_snapshot: typhoon_engine::core::research::RachevSnapshot,
    pub(crate) rachev_loading: bool,
    pub(crate) show_gpr: bool,
    pub(crate) gpr_symbol: String,
    pub(crate) gpr_snapshot: typhoon_engine::core::research::GprSnapshot,
    pub(crate) gpr_loading: bool,
    pub(crate) show_pacf: bool,
    pub(crate) pacf_symbol: String,
    pub(crate) pacf_snapshot: typhoon_engine::core::research::PacfSnapshot,
    pub(crate) pacf_loading: bool,
    pub(crate) show_apen: bool,
    pub(crate) apen_symbol: String,
    pub(crate) apen_snapshot: typhoon_engine::core::research::ApenSnapshot,
    pub(crate) apen_loading: bool,

    // ── Research section ──
    pub(crate) show_upr: bool,
    pub(crate) upr_symbol: String,
    pub(crate) upr_snapshot: typhoon_engine::core::research::UprSnapshot,
    pub(crate) upr_loading: bool,
    pub(crate) show_levereff: bool,
    pub(crate) levereff_symbol: String,
    pub(crate) levereff_snapshot: typhoon_engine::core::research::LeverEffSnapshot,
    pub(crate) levereff_loading: bool,
    pub(crate) show_drawdar: bool,
    pub(crate) drawdar_symbol: String,
    pub(crate) drawdar_snapshot: typhoon_engine::core::research::DrawDaRSnapshot,
    pub(crate) drawdar_loading: bool,
    pub(crate) show_varhalf: bool,
    pub(crate) varhalf_symbol: String,
    pub(crate) varhalf_snapshot: typhoon_engine::core::research::VarHalfSnapshot,
    pub(crate) varhalf_loading: bool,
    pub(crate) show_gini: bool,
    pub(crate) gini_symbol: String,
    pub(crate) gini_snapshot: typhoon_engine::core::research::GiniSnapshot,
    pub(crate) gini_loading: bool,
    // ── Research section ──
    pub(crate) show_sampen: bool,
    pub(crate) sampen_symbol: String,
    pub(crate) sampen_snapshot: typhoon_engine::core::research::SampenSnapshot,
    pub(crate) sampen_loading: bool,
    pub(crate) show_permen: bool,
    pub(crate) permen_symbol: String,
    pub(crate) permen_snapshot: typhoon_engine::core::research::PermenSnapshot,
    pub(crate) permen_loading: bool,
    pub(crate) show_recfact: bool,
    pub(crate) recfact_symbol: String,
    pub(crate) recfact_snapshot: typhoon_engine::core::research::RecfactSnapshot,
    pub(crate) recfact_loading: bool,
    pub(crate) show_kpss: bool,
    pub(crate) kpss_symbol: String,
    pub(crate) kpss_snapshot: typhoon_engine::core::research::KpssSnapshot,
    pub(crate) kpss_loading: bool,
    pub(crate) show_specent: bool,
    pub(crate) specent_symbol: String,
    pub(crate) specent_snapshot: typhoon_engine::core::research::SpecentSnapshot,
    pub(crate) specent_loading: bool,
    // ── Research section ──
    pub(crate) show_robvol: bool,
    pub(crate) robvol_symbol: String,
    pub(crate) robvol_snapshot: typhoon_engine::core::research::RobVolSnapshot,
    pub(crate) robvol_loading: bool,
    pub(crate) show_renyient: bool,
    pub(crate) renyient_symbol: String,
    pub(crate) renyient_snapshot: typhoon_engine::core::research::RenyientSnapshot,
    pub(crate) renyient_loading: bool,
    pub(crate) show_retquant: bool,
    pub(crate) retquant_symbol: String,
    pub(crate) retquant_snapshot: typhoon_engine::core::research::RetquantSnapshot,
    pub(crate) retquant_loading: bool,
    pub(crate) show_msent: bool,
    pub(crate) msent_symbol: String,
    pub(crate) msent_snapshot: typhoon_engine::core::research::MsentSnapshot,
    pub(crate) msent_loading: bool,
    pub(crate) show_ewmavol: bool,
    pub(crate) ewmavol_symbol: String,
    pub(crate) ewmavol_snapshot: typhoon_engine::core::research::EwmaVolSnapshot,
    pub(crate) ewmavol_loading: bool,
    // ── Research section ──
    pub(crate) show_ksnorm: bool,
    pub(crate) ksnorm_symbol: String,
    pub(crate) ksnorm_snapshot: typhoon_engine::core::research::KsnormSnapshot,
    pub(crate) ksnorm_loading: bool,
    pub(crate) show_adtest: bool,
    pub(crate) adtest_symbol: String,
    pub(crate) adtest_snapshot: typhoon_engine::core::research::AdtestSnapshot,
    pub(crate) adtest_loading: bool,
    pub(crate) show_lmom: bool,
    pub(crate) lmom_symbol: String,
    pub(crate) lmom_snapshot: typhoon_engine::core::research::LmomSnapshot,
    pub(crate) lmom_loading: bool,
    pub(crate) show_kylelam: bool,
    pub(crate) kylelam_symbol: String,
    pub(crate) kylelam_snapshot: typhoon_engine::core::research::KylelamSnapshot,
    pub(crate) kylelam_loading: bool,
    pub(crate) show_peakover: bool,
    pub(crate) peakover_symbol: String,
    pub(crate) peakover_snapshot: typhoon_engine::core::research::PeakoverSnapshot,
    pub(crate) peakover_loading: bool,
    // ── Research section ──
    pub(crate) show_higuchi: bool,
    pub(crate) higuchi_symbol: String,
    pub(crate) higuchi_snapshot: typhoon_engine::core::research::HiguchiSnapshot,
    pub(crate) higuchi_loading: bool,
    pub(crate) show_pickands: bool,
    pub(crate) pickands_symbol: String,
    pub(crate) pickands_snapshot: typhoon_engine::core::research::PickandsSnapshot,
    pub(crate) pickands_loading: bool,
    pub(crate) show_kappa3: bool,
    pub(crate) kappa3_symbol: String,
    pub(crate) kappa3_snapshot: typhoon_engine::core::research::Kappa3Snapshot,
    pub(crate) kappa3_loading: bool,
    pub(crate) show_lyapunov: bool,
    pub(crate) lyapunov_symbol: String,
    pub(crate) lyapunov_snapshot: typhoon_engine::core::research::LyapunovSnapshot,
    pub(crate) lyapunov_loading: bool,
    pub(crate) show_rankac: bool,
    pub(crate) rankac_symbol: String,
    pub(crate) rankac_snapshot: typhoon_engine::core::research::RankacSnapshot,
    pub(crate) rankac_loading: bool,
    // ── Research section ──
    pub(crate) show_bnsjump: bool,
    pub(crate) bnsjump_symbol: String,
    pub(crate) bnsjump_snapshot: typhoon_engine::core::research::BnsjumpSnapshot,
    pub(crate) bnsjump_loading: bool,
    pub(crate) show_pproot: bool,
    pub(crate) pproot_symbol: String,
    pub(crate) pproot_snapshot: typhoon_engine::core::research::PprootSnapshot,
    pub(crate) pproot_loading: bool,
    pub(crate) show_mfdfa: bool,
    pub(crate) mfdfa_symbol: String,
    pub(crate) mfdfa_snapshot: typhoon_engine::core::research::MfdfaSnapshot,
    pub(crate) mfdfa_loading: bool,
    pub(crate) show_hillks: bool,
    pub(crate) hillks_symbol: String,
    pub(crate) hillks_snapshot: typhoon_engine::core::research::HillksSnapshot,
    pub(crate) hillks_loading: bool,
    pub(crate) show_tsi: bool,
    pub(crate) tsi_symbol: String,
    pub(crate) tsi_snapshot: typhoon_engine::core::research::TsiSnapshot,
    pub(crate) tsi_loading: bool,
    // ── Research section ──
    pub(crate) show_garch11: bool,
    pub(crate) garch11_symbol: String,
    pub(crate) garch11_snapshot: typhoon_engine::core::research::Garch11Snapshot,
    pub(crate) garch11_loading: bool,
    pub(crate) show_sadf: bool,
    pub(crate) sadf_symbol: String,
    pub(crate) sadf_snapshot: typhoon_engine::core::research::SadfSnapshot,
    pub(crate) sadf_loading: bool,
    pub(crate) show_cordim: bool,
    pub(crate) cordim_symbol: String,
    pub(crate) cordim_snapshot: typhoon_engine::core::research::CordimSnapshot,
    pub(crate) cordim_loading: bool,
    pub(crate) show_skspec: bool,
    pub(crate) skspec_symbol: String,
    pub(crate) skspec_snapshot: typhoon_engine::core::research::SkspecSnapshot,
    pub(crate) skspec_loading: bool,
    pub(crate) show_automi: bool,
    pub(crate) automi_symbol: String,
    pub(crate) automi_snapshot: typhoon_engine::core::research::AutomiSnapshot,
    pub(crate) automi_loading: bool,
    // ── Research section ──
    pub(crate) show_durbinwatson: bool,
    pub(crate) durbinwatson_symbol: String,
    pub(crate) durbinwatson_snapshot: typhoon_engine::core::research::DurbinWatsonSnapshot,
    pub(crate) durbinwatson_loading: bool,
    pub(crate) show_bdstest: bool,
    pub(crate) bdstest_symbol: String,
    pub(crate) bdstest_snapshot: typhoon_engine::core::research::BdsTestSnapshot,
    pub(crate) bdstest_loading: bool,
    pub(crate) show_breuschpagan: bool,
    pub(crate) breuschpagan_symbol: String,
    pub(crate) breuschpagan_snapshot: typhoon_engine::core::research::BreuschPaganSnapshot,
    pub(crate) breuschpagan_loading: bool,
    pub(crate) show_turnpts: bool,
    pub(crate) turnpts_symbol: String,
    pub(crate) turnpts_snapshot: typhoon_engine::core::research::TurnPtsSnapshot,
    pub(crate) turnpts_loading: bool,
    pub(crate) show_periodogram: bool,
    pub(crate) periodogram_symbol: String,
    pub(crate) periodogram_snapshot: typhoon_engine::core::research::PeriodogramSnapshot,
    pub(crate) periodogram_loading: bool,
    // ── Research section ──
    pub(crate) show_mcleodli: bool,
    pub(crate) mcleodli_symbol: String,
    pub(crate) mcleodli_snapshot: typhoon_engine::core::research::McLeodLiSnapshot,
    pub(crate) mcleodli_loading: bool,
    pub(crate) show_oufit: bool,
    pub(crate) oufit_symbol: String,
    pub(crate) oufit_snapshot: typhoon_engine::core::research::OuFitSnapshot,
    pub(crate) oufit_loading: bool,
    pub(crate) show_gph: bool,
    pub(crate) gph_symbol: String,
    pub(crate) gph_snapshot: typhoon_engine::core::research::GphSnapshot,
    pub(crate) gph_loading: bool,
    pub(crate) show_burgspec: bool,
    pub(crate) burgspec_symbol: String,
    pub(crate) burgspec_snapshot: typhoon_engine::core::research::BurgSpecSnapshot,
    pub(crate) burgspec_loading: bool,
    pub(crate) show_kendalltau: bool,
    pub(crate) kendalltau_symbol: String,
    pub(crate) kendalltau_snapshot: typhoon_engine::core::research::KendallTauSnapshot,
    pub(crate) kendalltau_loading: bool,

    // ── Research section ──
    pub(crate) show_squeeze_win: bool,
    pub(crate) squeeze_win_symbol: String,
    pub(crate) squeeze_win_snapshot: typhoon_engine::core::research::SqueezeSnapshot,
    pub(crate) squeeze_win_loading: bool,
    pub(crate) show_squeezerank: bool,
    pub(crate) squeezerank_symbol: String,
    pub(crate) squeezerank_snapshot: typhoon_engine::core::research::SqueezeRankSnapshot,
    pub(crate) squeezerank_loading: bool,
    pub(crate) show_squeeze_watchlist: bool,
    pub(crate) squeeze_watchlist_rows: Vec<typhoon_engine::core::research::SqueezeSnapshot>,
    pub(crate) squeeze_watchlist_loading: bool,
    pub(crate) show_bbsqueeze: bool,
    pub(crate) bbsqueeze_symbol: String,
    pub(crate) bbsqueeze_snapshot: typhoon_engine::core::research::BbsqueezeSnapshot,
    pub(crate) bbsqueeze_loading: bool,
    pub(crate) show_donchian_win: bool,
    pub(crate) donchian_win_symbol: String,
    pub(crate) donchian_win_snapshot: typhoon_engine::core::research::DonchianSnapshot,
    pub(crate) donchian_win_loading: bool,
    pub(crate) show_kama_win: bool,
    pub(crate) kama_win_symbol: String,
    pub(crate) kama_win_snapshot: typhoon_engine::core::research::KamaSnapshot,
    pub(crate) kama_win_loading: bool,
    // ── Research section ──
    pub(crate) show_ichimoku_win: bool,
    pub(crate) ichimoku_win_symbol: String,
    pub(crate) ichimoku_win_snapshot: typhoon_engine::core::research::IchimokuSnapshot,
    pub(crate) ichimoku_win_loading: bool,
    pub(crate) show_supertrend_win: bool,
    pub(crate) supertrend_win_symbol: String,
    pub(crate) supertrend_win_snapshot: typhoon_engine::core::research::SupertrendSnapshot,
    pub(crate) supertrend_win_loading: bool,
    pub(crate) show_keltner_win: bool,
    pub(crate) keltner_win_symbol: String,
    pub(crate) keltner_win_snapshot: typhoon_engine::core::research::KeltnerSnapshot,
    pub(crate) keltner_win_loading: bool,
    pub(crate) show_fisher_win: bool,
    pub(crate) fisher_win_symbol: String,
    pub(crate) fisher_win_snapshot: typhoon_engine::core::research::FisherSnapshot,
    pub(crate) fisher_win_loading: bool,
    pub(crate) show_aroon_win: bool,
    pub(crate) aroon_win_symbol: String,
    pub(crate) aroon_win_snapshot: typhoon_engine::core::research::AroonSnapshot,
    pub(crate) aroon_win_loading: bool,
    // ── Research section ──
    pub(crate) show_adx_win: bool,
    pub(crate) adx_win_symbol: String,
    pub(crate) adx_win_snapshot: typhoon_engine::core::research::AdxSnapshot,
    pub(crate) adx_win_loading: bool,
    pub(crate) show_cci_win: bool,
    pub(crate) cci_win_symbol: String,
    pub(crate) cci_win_snapshot: typhoon_engine::core::research::CciSnapshot,
    pub(crate) cci_win_loading: bool,
    pub(crate) show_cmf_win: bool,
    pub(crate) cmf_win_symbol: String,
    pub(crate) cmf_win_snapshot: typhoon_engine::core::research::CmfSnapshot,
    pub(crate) cmf_win_loading: bool,
    pub(crate) show_mfi_win: bool,
    pub(crate) mfi_win_symbol: String,
    pub(crate) mfi_win_snapshot: typhoon_engine::core::research::MfiSnapshot,
    pub(crate) mfi_win_loading: bool,
    pub(crate) show_psar_win: bool,
    pub(crate) psar_win_symbol: String,
    pub(crate) psar_win_snapshot: typhoon_engine::core::research::PsarSnapshot,
    pub(crate) psar_win_loading: bool,
    // ── Research section ──
    pub(crate) show_vortex_win: bool,
    pub(crate) vortex_win_symbol: String,
    pub(crate) vortex_win_snapshot: typhoon_engine::core::research::VortexSnapshot,
    pub(crate) vortex_win_loading: bool,
    pub(crate) show_chop_win: bool,
    pub(crate) chop_win_symbol: String,
    pub(crate) chop_win_snapshot: typhoon_engine::core::research::ChopSnapshot,
    pub(crate) chop_win_loading: bool,
    pub(crate) show_obv_win: bool,
    pub(crate) obv_win_symbol: String,
    pub(crate) obv_win_snapshot: typhoon_engine::core::research::ObvSnapshot,
    pub(crate) obv_win_loading: bool,
    pub(crate) show_trix_win: bool,
    pub(crate) trix_win_symbol: String,
    pub(crate) trix_win_snapshot: typhoon_engine::core::research::TrixSnapshot,
    pub(crate) trix_win_loading: bool,
    pub(crate) show_hma_win: bool,
    pub(crate) hma_win_symbol: String,
    pub(crate) hma_win_snapshot: typhoon_engine::core::research::HmaSnapshot,
    pub(crate) hma_win_loading: bool,
    // ── Research section ──
    pub(crate) show_ppo_win: bool,
    pub(crate) ppo_win_symbol: String,
    pub(crate) ppo_win_snapshot: typhoon_engine::core::research::PpoSnapshot,
    pub(crate) ppo_win_loading: bool,
    pub(crate) show_dpo_win: bool,
    pub(crate) dpo_win_symbol: String,
    pub(crate) dpo_win_snapshot: typhoon_engine::core::research::DpoSnapshot,
    pub(crate) dpo_win_loading: bool,
    pub(crate) show_kst_win: bool,
    pub(crate) kst_win_symbol: String,
    pub(crate) kst_win_snapshot: typhoon_engine::core::research::KstSnapshot,
    pub(crate) kst_win_loading: bool,
    pub(crate) show_ultosc_win: bool,
    pub(crate) ultosc_win_symbol: String,
    pub(crate) ultosc_win_snapshot: typhoon_engine::core::research::UltoscSnapshot,
    pub(crate) ultosc_win_loading: bool,
    pub(crate) show_willr_win: bool,
    pub(crate) willr_win_symbol: String,
    pub(crate) willr_win_snapshot: typhoon_engine::core::research::WillrSnapshot,
    pub(crate) willr_win_loading: bool,
    // ── Research section ──
    pub(crate) show_mass_win: bool,
    pub(crate) mass_win_symbol: String,
    pub(crate) mass_win_snapshot: typhoon_engine::core::research::MassSnapshot,
    pub(crate) mass_win_loading: bool,
    pub(crate) show_chaikosc_win: bool,
    pub(crate) chaikosc_win_symbol: String,
    pub(crate) chaikosc_win_snapshot: typhoon_engine::core::research::ChaikoscSnapshot,
    pub(crate) chaikosc_win_loading: bool,
    pub(crate) show_klinger_win: bool,
    pub(crate) klinger_win_symbol: String,
    pub(crate) klinger_win_snapshot: typhoon_engine::core::research::KlingerSnapshot,
    pub(crate) klinger_win_loading: bool,
    pub(crate) show_stochrsi_win: bool,
    pub(crate) stochrsi_win_symbol: String,
    pub(crate) stochrsi_win_snapshot: typhoon_engine::core::research::StochRsiSnapshot,
    pub(crate) stochrsi_win_loading: bool,
    pub(crate) show_awesome_win: bool,
    pub(crate) awesome_win_symbol: String,
    pub(crate) awesome_win_snapshot: typhoon_engine::core::research::AwesomeSnapshot,
    pub(crate) awesome_win_loading: bool,
    // ── Research section ──
    pub(crate) show_efi_win: bool,
    pub(crate) efi_win_symbol: String,
    pub(crate) efi_win_snapshot: typhoon_engine::core::research::EfiSnapshot,
    pub(crate) efi_win_loading: bool,
    pub(crate) show_emv_win: bool,
    pub(crate) emv_win_symbol: String,
    pub(crate) emv_win_snapshot: typhoon_engine::core::research::EmvSnapshot,
    pub(crate) emv_win_loading: bool,
    pub(crate) show_nvi_win: bool,
    pub(crate) nvi_win_symbol: String,
    pub(crate) nvi_win_snapshot: typhoon_engine::core::research::NviSnapshot,
    pub(crate) nvi_win_loading: bool,
    pub(crate) show_pvi_win: bool,
    pub(crate) pvi_win_symbol: String,
    pub(crate) pvi_win_snapshot: typhoon_engine::core::research::PviSnapshot,
    pub(crate) pvi_win_loading: bool,
    pub(crate) show_coppock_win: bool,
    pub(crate) coppock_win_symbol: String,
    pub(crate) coppock_win_snapshot: typhoon_engine::core::research::CoppockSnapshot,
    pub(crate) coppock_win_loading: bool,
    // ── Research section ──
    pub(crate) show_cmo_win: bool,
    pub(crate) cmo_win_symbol: String,
    pub(crate) cmo_win_snapshot: typhoon_engine::core::research::CmoSnapshot,
    pub(crate) cmo_win_loading: bool,
    pub(crate) show_qstick_win: bool,
    pub(crate) qstick_win_symbol: String,
    pub(crate) qstick_win_snapshot: typhoon_engine::core::research::QstickSnapshot,
    pub(crate) qstick_win_loading: bool,
    pub(crate) show_disparity_win: bool,
    pub(crate) disparity_win_symbol: String,
    pub(crate) disparity_win_snapshot: typhoon_engine::core::research::DisparitySnapshot,
    pub(crate) disparity_win_loading: bool,
    pub(crate) show_bop_win: bool,
    pub(crate) bop_win_symbol: String,
    pub(crate) bop_win_snapshot: typhoon_engine::core::research::BopSnapshot,
    pub(crate) bop_win_loading: bool,
    pub(crate) show_schaff_win: bool,
    pub(crate) schaff_win_symbol: String,
    pub(crate) schaff_win_snapshot: typhoon_engine::core::research::SchaffSnapshot,
    pub(crate) schaff_win_loading: bool,
    // ── Research section ──
    pub(crate) show_stoch_win: bool,
    pub(crate) stoch_win_symbol: String,
    pub(crate) stoch_win_snapshot: typhoon_engine::core::research::StochSnapshot,
    pub(crate) stoch_win_loading: bool,
    pub(crate) show_macd_win: bool,
    pub(crate) macd_win_symbol: String,
    pub(crate) macd_win_snapshot: typhoon_engine::core::research::MacdSnapshot,
    pub(crate) macd_win_loading: bool,
    pub(crate) show_vwap_win: bool,
    pub(crate) vwap_win_symbol: String,
    pub(crate) vwap_win_snapshot: typhoon_engine::core::research::VwapSnapshot,
    pub(crate) vwap_win_loading: bool,
    pub(crate) show_mcgd_win: bool,
    pub(crate) mcgd_win_symbol: String,
    pub(crate) mcgd_win_snapshot: typhoon_engine::core::research::McgdSnapshot,
    pub(crate) mcgd_win_loading: bool,
    pub(crate) show_rwi_win: bool,
    pub(crate) rwi_win_symbol: String,
    pub(crate) rwi_win_snapshot: typhoon_engine::core::research::RwiSnapshot,
    pub(crate) rwi_win_loading: bool,
    // ── Research section ──
    pub(crate) show_dema_win: bool,
    pub(crate) dema_win_symbol: String,
    pub(crate) dema_win_snapshot: typhoon_engine::core::research::DemaSnapshot,
    pub(crate) dema_win_loading: bool,
    pub(crate) show_tema_win: bool,
    pub(crate) tema_win_symbol: String,
    pub(crate) tema_win_snapshot: typhoon_engine::core::research::TemaSnapshot,
    pub(crate) tema_win_loading: bool,
    pub(crate) show_linreg_win: bool,
    pub(crate) linreg_win_symbol: String,
    pub(crate) linreg_win_snapshot: typhoon_engine::core::research::LinregSnapshot,
    pub(crate) linreg_win_loading: bool,
    pub(crate) show_pivots_win: bool,
    pub(crate) pivots_win_symbol: String,
    pub(crate) pivots_win_snapshot: typhoon_engine::core::research::PivotsSnapshot,
    pub(crate) pivots_win_loading: bool,
    pub(crate) show_heikin_win: bool,
    pub(crate) heikin_win_symbol: String,
    pub(crate) heikin_win_snapshot: typhoon_engine::core::research::HeikinSnapshot,
    pub(crate) heikin_win_loading: bool,
    // ── Research section ──
    pub(crate) show_alma_win: bool,
    pub(crate) alma_win_symbol: String,
    pub(crate) alma_win_snapshot: typhoon_engine::core::research::AlmaSnapshot,
    pub(crate) alma_win_loading: bool,
    pub(crate) show_zlema_win: bool,
    pub(crate) zlema_win_symbol: String,
    pub(crate) zlema_win_snapshot: typhoon_engine::core::research::ZlemaSnapshot,
    pub(crate) zlema_win_loading: bool,
    pub(crate) show_elderray_win: bool,
    pub(crate) elderray_win_symbol: String,
    pub(crate) elderray_win_snapshot: typhoon_engine::core::research::ElderRaySnapshot,
    pub(crate) elderray_win_loading: bool,
    pub(crate) show_tsf_win: bool,
    pub(crate) tsf_win_symbol: String,
    pub(crate) tsf_win_snapshot: typhoon_engine::core::research::TsfSnapshot,
    pub(crate) tsf_win_loading: bool,
    pub(crate) show_rvi_win: bool,
    pub(crate) rvi_win_symbol: String,
    pub(crate) rvi_win_snapshot: typhoon_engine::core::research::RviSnapshot,
    pub(crate) rvi_win_loading: bool,
    // ── Research section ──
    pub(crate) show_trima_win: bool,
    pub(crate) trima_win_symbol: String,
    pub(crate) trima_win_snapshot: typhoon_engine::core::research::TrimaSnapshot,
    pub(crate) trima_win_loading: bool,
    pub(crate) show_t3_win: bool,
    pub(crate) t3_win_symbol: String,
    pub(crate) t3_win_snapshot: typhoon_engine::core::research::T3Snapshot,
    pub(crate) t3_win_loading: bool,
    pub(crate) show_vidya_win: bool,
    pub(crate) vidya_win_symbol: String,
    pub(crate) vidya_win_snapshot: typhoon_engine::core::research::VidyaSnapshot,
    pub(crate) vidya_win_loading: bool,
    pub(crate) show_smi_win: bool,
    pub(crate) smi_win_symbol: String,
    pub(crate) smi_win_snapshot: typhoon_engine::core::research::SmiSnapshot,
    pub(crate) smi_win_loading: bool,
    pub(crate) show_pvt_win: bool,
    pub(crate) pvt_win_symbol: String,
    pub(crate) pvt_win_snapshot: typhoon_engine::core::research::PvtSnapshot,
    pub(crate) pvt_win_loading: bool,
    // ── Research section ──
    pub(crate) show_ac_win: bool,
    pub(crate) ac_win_symbol: String,
    pub(crate) ac_win_snapshot: typhoon_engine::core::research::AcSnapshot,
    pub(crate) ac_win_loading: bool,
    pub(crate) show_chvol_win: bool,
    pub(crate) chvol_win_symbol: String,
    pub(crate) chvol_win_snapshot: typhoon_engine::core::research::ChvolSnapshot,
    pub(crate) chvol_win_loading: bool,
    pub(crate) show_bbwidth_win: bool,
    pub(crate) bbwidth_win_symbol: String,
    pub(crate) bbwidth_win_snapshot: typhoon_engine::core::research::BbwidthSnapshot,
    pub(crate) bbwidth_win_loading: bool,
    pub(crate) show_elderimp_win: bool,
    pub(crate) elderimp_win_symbol: String,
    pub(crate) elderimp_win_snapshot: typhoon_engine::core::research::ElderImpulseSnapshot,
    pub(crate) elderimp_win_loading: bool,
    pub(crate) show_rmi_win: bool,
    pub(crate) rmi_win_symbol: String,
    pub(crate) rmi_win_snapshot: typhoon_engine::core::research::RmiSnapshot,
    pub(crate) rmi_win_loading: bool,

    // ── Options Expiration Calendar ──
    pub(crate) show_expcal_win: bool,
    pub(crate) expcal_win_symbol: String,
    pub(crate) expcal_win_snapshot: typhoon_engine::core::research::SymbolExpirationsSnapshot,
    pub(crate) expcal_win_loading: bool,
    pub(crate) expcal_win_tab: u8, // 0 = Tier 1 market calendar, 1 = Tier 2 symbol chain
    pub(crate) expcal_win_horizon_days: u32,
    pub(crate) expcal_win_calendar: Vec<typhoon_engine::core::research::CalendarExpiry>,

    // ── SMMA / ALLIGATOR / CRSI / SEB / IMI ──
    pub(crate) show_smma_win: bool,
    pub(crate) smma_win_symbol: String,
    pub(crate) smma_win_snapshot: typhoon_engine::core::research::SmmaSnapshot,
    pub(crate) smma_win_loading: bool,
    pub(crate) show_alligator_win: bool,
    pub(crate) alligator_win_symbol: String,
    pub(crate) alligator_win_snapshot: typhoon_engine::core::research::AlligatorSnapshot,
    pub(crate) alligator_win_loading: bool,
    pub(crate) show_crsi_win: bool,
    pub(crate) crsi_win_symbol: String,
    pub(crate) crsi_win_snapshot: typhoon_engine::core::research::CrsiSnapshot,
    pub(crate) crsi_win_loading: bool,
    pub(crate) show_seb_win: bool,
    pub(crate) seb_win_symbol: String,
    pub(crate) seb_win_snapshot: typhoon_engine::core::research::SebSnapshot,
    pub(crate) seb_win_loading: bool,
    pub(crate) show_imi_win: bool,
    pub(crate) imi_win_symbol: String,
    pub(crate) imi_win_snapshot: typhoon_engine::core::research::ImiSnapshot,
    pub(crate) imi_win_loading: bool,

    // ── GMMA / MAENV / ADL / VHF / VROC ──
    pub(crate) show_gmma_win: bool,
    pub(crate) gmma_win_symbol: String,
    pub(crate) gmma_win_snapshot: typhoon_engine::core::research::GmmaSnapshot,
    pub(crate) gmma_win_loading: bool,
    pub(crate) show_maenv_win: bool,
    pub(crate) maenv_win_symbol: String,
    pub(crate) maenv_win_snapshot: typhoon_engine::core::research::MaenvSnapshot,
    pub(crate) maenv_win_loading: bool,
    pub(crate) show_adl_win: bool,
    pub(crate) adl_win_symbol: String,
    pub(crate) adl_win_snapshot: typhoon_engine::core::research::AdlSnapshot,
    pub(crate) adl_win_loading: bool,
    pub(crate) show_vhf_win: bool,
    pub(crate) vhf_win_symbol: String,
    pub(crate) vhf_win_snapshot: typhoon_engine::core::research::VhfSnapshot,
    pub(crate) vhf_win_loading: bool,
    pub(crate) show_vroc_win: bool,
    pub(crate) vroc_win_symbol: String,
    pub(crate) vroc_win_snapshot: typhoon_engine::core::research::VrocSnapshot,
    pub(crate) vroc_win_loading: bool,

    // ── KDJ / QQE / PMO / CFO / TMF ──
    pub(crate) show_kdj_win: bool,
    pub(crate) kdj_win_symbol: String,
    pub(crate) kdj_win_snapshot: typhoon_engine::core::research::KdjSnapshot,
    pub(crate) kdj_win_loading: bool,
    pub(crate) show_qqe_win: bool,
    pub(crate) qqe_win_symbol: String,
    pub(crate) qqe_win_snapshot: typhoon_engine::core::research::QqeSnapshot,
    pub(crate) qqe_win_loading: bool,
    pub(crate) show_pmo_win: bool,
    pub(crate) pmo_win_symbol: String,
    pub(crate) pmo_win_snapshot: typhoon_engine::core::research::PmoSnapshot,
    pub(crate) pmo_win_loading: bool,
    pub(crate) show_cfo_win: bool,
    pub(crate) cfo_win_symbol: String,
    pub(crate) cfo_win_snapshot: typhoon_engine::core::research::CfoSnapshot,
    pub(crate) cfo_win_loading: bool,
    pub(crate) show_tmf_win: bool,
    pub(crate) tmf_win_symbol: String,
    pub(crate) tmf_win_snapshot: typhoon_engine::core::research::TmfSnapshot,
    pub(crate) tmf_win_loading: bool,

    // ── FRACTALS / IFT_RSI / MAMA / COG / DIDI ──
    pub(crate) show_fractals_win: bool,
    pub(crate) fractals_win_symbol: String,
    pub(crate) fractals_win_snapshot: typhoon_engine::core::research::FractalsSnapshot,
    pub(crate) fractals_win_loading: bool,
    pub(crate) show_ift_rsi_win: bool,
    pub(crate) ift_rsi_win_symbol: String,
    pub(crate) ift_rsi_win_snapshot: typhoon_engine::core::research::IftRsiSnapshot,
    pub(crate) ift_rsi_win_loading: bool,
    pub(crate) show_mama_win: bool,
    pub(crate) mama_win_symbol: String,
    pub(crate) mama_win_snapshot: typhoon_engine::core::research::MamaSnapshot,
    pub(crate) mama_win_loading: bool,
    pub(crate) show_cog_win: bool,
    pub(crate) cog_win_symbol: String,
    pub(crate) cog_win_snapshot: typhoon_engine::core::research::CogSnapshot,
    pub(crate) cog_win_loading: bool,
    pub(crate) show_didi_win: bool,
    pub(crate) didi_win_symbol: String,
    pub(crate) didi_win_snapshot: typhoon_engine::core::research::DidiSnapshot,
    pub(crate) didi_win_loading: bool,

    // ── DEMARKER / GATOR / BW_MFI / VWMA / STDDEV ──
    pub(crate) show_demarker_win: bool,
    pub(crate) demarker_win_symbol: String,
    pub(crate) demarker_win_snapshot: typhoon_engine::core::research::DemarkerSnapshot,
    pub(crate) demarker_win_loading: bool,
    pub(crate) show_gator_win: bool,
    pub(crate) gator_win_symbol: String,
    pub(crate) gator_win_snapshot: typhoon_engine::core::research::GatorSnapshot,
    pub(crate) gator_win_loading: bool,
    pub(crate) show_bw_mfi_win: bool,
    pub(crate) bw_mfi_win_symbol: String,
    pub(crate) bw_mfi_win_snapshot: typhoon_engine::core::research::BwMfiSnapshot,
    pub(crate) bw_mfi_win_loading: bool,
    pub(crate) show_vwma_win: bool,
    pub(crate) vwma_win_symbol: String,
    pub(crate) vwma_win_snapshot: typhoon_engine::core::research::VwmaSnapshot,
    pub(crate) vwma_win_loading: bool,
    pub(crate) show_stddev_win: bool,
    pub(crate) stddev_win_symbol: String,
    pub(crate) stddev_win_snapshot: typhoon_engine::core::research::StddevSnapshot,
    pub(crate) stddev_win_loading: bool,

    // ── WMA / RAINBOW / MESA_SINE / FRAMA / IBS ──
    pub(crate) show_wma_win: bool,
    pub(crate) wma_win_symbol: String,
    pub(crate) wma_win_snapshot: typhoon_engine::core::research::WmaSnapshot,
    pub(crate) wma_win_loading: bool,
    pub(crate) show_rainbow_win: bool,
    pub(crate) rainbow_win_symbol: String,
    pub(crate) rainbow_win_snapshot: typhoon_engine::core::research::RainbowSnapshot,
    pub(crate) rainbow_win_loading: bool,
    pub(crate) show_mesa_sine_win: bool,
    pub(crate) mesa_sine_win_symbol: String,
    pub(crate) mesa_sine_win_snapshot: typhoon_engine::core::research::MesaSineSnapshot,
    pub(crate) mesa_sine_win_loading: bool,
    pub(crate) show_frama_win: bool,
    pub(crate) frama_win_symbol: String,
    pub(crate) frama_win_snapshot: typhoon_engine::core::research::FramaSnapshot,
    pub(crate) frama_win_loading: bool,
    pub(crate) show_ibs_win: bool,
    pub(crate) ibs_win_symbol: String,
    pub(crate) ibs_win_snapshot: typhoon_engine::core::research::IbsSnapshot,
    pub(crate) ibs_win_loading: bool,

    // ── LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT ──
    pub(crate) show_laguerre_rsi_win: bool,
    pub(crate) laguerre_rsi_win_symbol: String,
    pub(crate) laguerre_rsi_win_snapshot: typhoon_engine::core::research::LaguerreRsiSnapshot,
    pub(crate) laguerre_rsi_win_loading: bool,
    pub(crate) show_zigzag_win: bool,
    pub(crate) zigzag_win_symbol: String,
    pub(crate) zigzag_win_snapshot: typhoon_engine::core::research::ZigzagSnapshot,
    pub(crate) zigzag_win_loading: bool,
    pub(crate) show_pgo_win: bool,
    pub(crate) pgo_win_symbol: String,
    pub(crate) pgo_win_snapshot: typhoon_engine::core::research::PgoSnapshot,
    pub(crate) pgo_win_loading: bool,
    pub(crate) show_ht_trendline_win: bool,
    pub(crate) ht_trendline_win_symbol: String,
    pub(crate) ht_trendline_win_snapshot: typhoon_engine::core::research::HtTrendlineSnapshot,
    pub(crate) ht_trendline_win_loading: bool,
    pub(crate) show_midpoint_win: bool,
    pub(crate) midpoint_win_symbol: String,
    pub(crate) midpoint_win_snapshot: typhoon_engine::core::research::MidpointSnapshot,
    pub(crate) midpoint_win_loading: bool,

    // ── MASSINDEX / NATR / TTM_SQUEEZE / FORCE_INDEX / TRANGE ──
    pub(crate) show_mass_index_win: bool,
    pub(crate) mass_index_win_symbol: String,
    pub(crate) mass_index_win_snapshot: typhoon_engine::core::research::MassIndexSnapshot,
    pub(crate) mass_index_win_loading: bool,
    pub(crate) show_natr_win: bool,
    pub(crate) natr_win_symbol: String,
    pub(crate) natr_win_snapshot: typhoon_engine::core::research::NatrSnapshot,
    pub(crate) natr_win_loading: bool,
    pub(crate) show_ttm_squeeze_win: bool,
    pub(crate) ttm_squeeze_win_symbol: String,
    pub(crate) ttm_squeeze_win_snapshot: typhoon_engine::core::research::TtmSqueezeSnapshot,
    pub(crate) ttm_squeeze_win_loading: bool,
    pub(crate) show_force_index_win: bool,
    pub(crate) force_index_win_symbol: String,
    pub(crate) force_index_win_snapshot: typhoon_engine::core::research::ForceIndexSnapshot,
    pub(crate) force_index_win_loading: bool,
    pub(crate) show_trange_win: bool,
    pub(crate) trange_win_symbol: String,
    pub(crate) trange_win_snapshot: typhoon_engine::core::research::TrangeSnapshot,
    pub(crate) trange_win_loading: bool,

    // ── LINEARREG_SLOPE / HT_DCPERIOD / HT_TRENDMODE / ACCBANDS / STOCHF ──
    pub(crate) show_linearreg_slope_win: bool,
    pub(crate) linearreg_slope_win_symbol: String,
    pub(crate) linearreg_slope_win_snapshot: typhoon_engine::core::research::LinearregSlopeSnapshot,
    pub(crate) linearreg_slope_win_loading: bool,
    pub(crate) show_ht_dcperiod_win: bool,
    pub(crate) ht_dcperiod_win_symbol: String,
    pub(crate) ht_dcperiod_win_snapshot: typhoon_engine::core::research::HtDcperiodSnapshot,
    pub(crate) ht_dcperiod_win_loading: bool,
    pub(crate) show_ht_trendmode_win: bool,
    pub(crate) ht_trendmode_win_symbol: String,
    pub(crate) ht_trendmode_win_snapshot: typhoon_engine::core::research::HtTrendmodeSnapshot,
    pub(crate) ht_trendmode_win_loading: bool,
    pub(crate) show_accbands_win: bool,
    pub(crate) accbands_win_symbol: String,
    pub(crate) accbands_win_snapshot: typhoon_engine::core::research::AccbandsSnapshot,
    pub(crate) accbands_win_loading: bool,
    pub(crate) show_stochf_win: bool,
    pub(crate) stochf_win_symbol: String,
    pub(crate) stochf_win_snapshot: typhoon_engine::core::research::StochfSnapshot,
    pub(crate) stochf_win_loading: bool,

    // ── LINEARREG / LINEARREG_ANGLE / HT_DCPHASE / HT_SINE / HT_PHASOR ──
    pub(crate) show_linearreg_win: bool,
    pub(crate) linearreg_win_symbol: String,
    pub(crate) linearreg_win_snapshot: typhoon_engine::core::research::LinearregSnapshot,
    pub(crate) linearreg_win_loading: bool,
    pub(crate) show_linearreg_angle_win: bool,
    pub(crate) linearreg_angle_win_symbol: String,
    pub(crate) linearreg_angle_win_snapshot: typhoon_engine::core::research::LinearregAngleSnapshot,
    pub(crate) linearreg_angle_win_loading: bool,
    pub(crate) show_ht_dcphase_win: bool,
    pub(crate) ht_dcphase_win_symbol: String,
    pub(crate) ht_dcphase_win_snapshot: typhoon_engine::core::research::HtDcphaseSnapshot,
    pub(crate) ht_dcphase_win_loading: bool,
    pub(crate) show_ht_sine_win: bool,
    pub(crate) ht_sine_win_symbol: String,
    pub(crate) ht_sine_win_snapshot: typhoon_engine::core::research::HtSineSnapshot,
    pub(crate) ht_sine_win_loading: bool,
    pub(crate) show_ht_phasor_win: bool,
    pub(crate) ht_phasor_win_symbol: String,
    pub(crate) ht_phasor_win_snapshot: typhoon_engine::core::research::HtPhasorSnapshot,
    pub(crate) ht_phasor_win_loading: bool,

    // ── MIDPRICE / APO / MOM / SAREXT / ADXR ──
    pub(crate) show_midprice_win: bool,
    pub(crate) midprice_win_symbol: String,
    pub(crate) midprice_win_snapshot: typhoon_engine::core::research::MidpriceSnapshot,
    pub(crate) midprice_win_loading: bool,
    pub(crate) show_apo_win: bool,
    pub(crate) apo_win_symbol: String,
    pub(crate) apo_win_snapshot: typhoon_engine::core::research::ApoSnapshot,
    pub(crate) apo_win_loading: bool,
    pub(crate) show_mom_win: bool,
    pub(crate) mom_win_symbol: String,
    pub(crate) mom_win_snapshot: typhoon_engine::core::research::MomSnapshot,
    pub(crate) mom_win_loading: bool,
    pub(crate) show_sarext_win: bool,
    pub(crate) sarext_win_symbol: String,
    pub(crate) sarext_win_snapshot: typhoon_engine::core::research::SarextSnapshot,
    pub(crate) sarext_win_loading: bool,
    pub(crate) show_adxr_win: bool,
    pub(crate) adxr_win_symbol: String,
    pub(crate) adxr_win_snapshot: typhoon_engine::core::research::AdxrSnapshot,
    pub(crate) adxr_win_loading: bool,

    // ── AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
    pub(crate) show_avgprice_win: bool,
    pub(crate) avgprice_win_symbol: String,
    pub(crate) avgprice_win_snapshot: typhoon_engine::core::research::AvgpriceSnapshot,
    pub(crate) avgprice_win_loading: bool,
    pub(crate) show_medprice_win: bool,
    pub(crate) medprice_win_symbol: String,
    pub(crate) medprice_win_snapshot: typhoon_engine::core::research::MedpriceSnapshot,
    pub(crate) medprice_win_loading: bool,
    pub(crate) show_typprice_win: bool,
    pub(crate) typprice_win_symbol: String,
    pub(crate) typprice_win_snapshot: typhoon_engine::core::research::TypPriceSnapshot,
    pub(crate) typprice_win_loading: bool,
    pub(crate) show_wclprice_win: bool,
    pub(crate) wclprice_win_symbol: String,
    pub(crate) wclprice_win_snapshot: typhoon_engine::core::research::WclPriceSnapshot,
    pub(crate) wclprice_win_loading: bool,
    pub(crate) show_variance_win: bool,
    pub(crate) variance_win_symbol: String,
    pub(crate) variance_win_snapshot: typhoon_engine::core::research::VarianceSnapshot,
    pub(crate) variance_win_loading: bool,
    // ── Research section ──
    pub(crate) show_plus_di_win: bool,
    pub(crate) plus_di_win_symbol: String,
    pub(crate) plus_di_win_snapshot: typhoon_engine::core::research::PlusDiSnapshot,
    pub(crate) plus_di_win_loading: bool,
    pub(crate) show_minus_di_win: bool,
    pub(crate) minus_di_win_symbol: String,
    pub(crate) minus_di_win_snapshot: typhoon_engine::core::research::MinusDiSnapshot,
    pub(crate) minus_di_win_loading: bool,
    pub(crate) show_plus_dm_win: bool,
    pub(crate) plus_dm_win_symbol: String,
    pub(crate) plus_dm_win_snapshot: typhoon_engine::core::research::PlusDmSnapshot,
    pub(crate) plus_dm_win_loading: bool,
    pub(crate) show_minus_dm_win: bool,
    pub(crate) minus_dm_win_symbol: String,
    pub(crate) minus_dm_win_snapshot: typhoon_engine::core::research::MinusDmSnapshot,
    pub(crate) minus_dm_win_loading: bool,
    pub(crate) show_dx_win: bool,
    pub(crate) dx_win_symbol: String,
    pub(crate) dx_win_snapshot: typhoon_engine::core::research::DxSnapshot,
    pub(crate) dx_win_loading: bool,
    // ── Research section ──
    pub(crate) show_roc_win: bool,
    pub(crate) roc_win_symbol: String,
    pub(crate) roc_win_snapshot: typhoon_engine::core::research::RocSnapshot,
    pub(crate) roc_win_loading: bool,
    pub(crate) show_rocp_win: bool,
    pub(crate) rocp_win_symbol: String,
    pub(crate) rocp_win_snapshot: typhoon_engine::core::research::RocpSnapshot,
    pub(crate) rocp_win_loading: bool,
    pub(crate) show_rocr_win: bool,
    pub(crate) rocr_win_symbol: String,
    pub(crate) rocr_win_snapshot: typhoon_engine::core::research::RocrSnapshot,
    pub(crate) rocr_win_loading: bool,
    pub(crate) show_rocr100_win: bool,
    pub(crate) rocr100_win_symbol: String,
    pub(crate) rocr100_win_snapshot: typhoon_engine::core::research::Rocr100Snapshot,
    pub(crate) rocr100_win_loading: bool,
    pub(crate) show_correl_win: bool,
    pub(crate) correl_win_symbol: String,
    pub(crate) correl_win_snapshot: typhoon_engine::core::research::CorrelSnapshot,
    pub(crate) correl_win_loading: bool,
    // ── Research section ──
    pub(crate) show_min_win: bool,
    pub(crate) min_win_symbol: String,
    pub(crate) min_win_snapshot: typhoon_engine::core::research::MinSnapshot,
    pub(crate) min_win_loading: bool,
    pub(crate) show_max_win: bool,
    pub(crate) max_win_symbol: String,
    pub(crate) max_win_snapshot: typhoon_engine::core::research::MaxSnapshot,
    pub(crate) max_win_loading: bool,
    pub(crate) show_minmax_win: bool,
    pub(crate) minmax_win_symbol: String,
    pub(crate) minmax_win_snapshot: typhoon_engine::core::research::MinMaxSnapshot,
    pub(crate) minmax_win_loading: bool,
    pub(crate) show_minindex_win: bool,
    pub(crate) minindex_win_symbol: String,
    pub(crate) minindex_win_snapshot: typhoon_engine::core::research::MinIndexSnapshot,
    pub(crate) minindex_win_loading: bool,
    pub(crate) show_maxindex_win: bool,
    pub(crate) maxindex_win_symbol: String,
    pub(crate) maxindex_win_snapshot: typhoon_engine::core::research::MaxIndexSnapshot,
    pub(crate) maxindex_win_loading: bool,
    // ── Research section ──
    pub(crate) show_bbands_win: bool,
    pub(crate) bbands_win_symbol: String,
    pub(crate) bbands_win_snapshot: typhoon_engine::core::research::BbandsSnapshot,
    pub(crate) bbands_win_loading: bool,
    pub(crate) show_ad_win: bool,
    pub(crate) ad_win_symbol: String,
    pub(crate) ad_win_snapshot: typhoon_engine::core::research::AdSnapshot,
    pub(crate) ad_win_loading: bool,
    pub(crate) show_adosc_win: bool,
    pub(crate) adosc_win_symbol: String,
    pub(crate) adosc_win_snapshot: typhoon_engine::core::research::AdoscSnapshot,
    pub(crate) adosc_win_loading: bool,
    pub(crate) show_sum_win: bool,
    pub(crate) sum_win_symbol: String,
    pub(crate) sum_win_snapshot: typhoon_engine::core::research::SumSnapshot,
    pub(crate) sum_win_loading: bool,
    pub(crate) show_linreg_intercept_win: bool,
    pub(crate) linreg_intercept_win_symbol: String,
    pub(crate) linreg_intercept_win_snapshot:
        typhoon_engine::core::research::LinearRegInterceptSnapshot,
    pub(crate) linreg_intercept_win_loading: bool,
    // ── AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP ──
    pub(crate) show_aroonosc_win: bool,
    pub(crate) aroonosc_win_symbol: String,
    pub(crate) aroonosc_win_snapshot: typhoon_engine::core::research::AroonoscSnapshot,
    pub(crate) aroonosc_win_loading: bool,
    pub(crate) show_minmaxindex_win: bool,
    pub(crate) minmaxindex_win_symbol: String,
    pub(crate) minmaxindex_win_snapshot: typhoon_engine::core::research::MinMaxIndexSnapshot,
    pub(crate) minmaxindex_win_loading: bool,
    pub(crate) show_macdext_win: bool,
    pub(crate) macdext_win_symbol: String,
    pub(crate) macdext_win_snapshot: typhoon_engine::core::research::MacdextSnapshot,
    pub(crate) macdext_win_loading: bool,
    pub(crate) show_macdfix_win: bool,
    pub(crate) macdfix_win_symbol: String,
    pub(crate) macdfix_win_snapshot: typhoon_engine::core::research::MacdfixSnapshot,
    pub(crate) macdfix_win_loading: bool,
    pub(crate) show_mavp_win: bool,
    pub(crate) mavp_win_symbol: String,
    pub(crate) mavp_win_snapshot: typhoon_engine::core::research::MavpSnapshot,
    pub(crate) mavp_win_loading: bool,
    // Candlestick pattern storage/helpers
    pub(crate) show_cdl_doji_win: bool,
    pub(crate) cdl_doji_win_symbol: String,
    pub(crate) cdl_doji_win_snapshot: typhoon_engine::core::research::CdlDojiSnapshot,
    pub(crate) cdl_doji_win_loading: bool,
    pub(crate) show_cdl_hammer_win: bool,
    pub(crate) cdl_hammer_win_symbol: String,
    pub(crate) cdl_hammer_win_snapshot: typhoon_engine::core::research::CdlHammerSnapshot,
    pub(crate) cdl_hammer_win_loading: bool,
    pub(crate) show_cdl_shooting_star_win: bool,
    pub(crate) cdl_shooting_star_win_symbol: String,
    pub(crate) cdl_shooting_star_win_snapshot:
        typhoon_engine::core::research::CdlShootingStarSnapshot,
    pub(crate) cdl_shooting_star_win_loading: bool,
    pub(crate) show_cdl_engulfing_win: bool,
    pub(crate) cdl_engulfing_win_symbol: String,
    pub(crate) cdl_engulfing_win_snapshot: typhoon_engine::core::research::CdlEngulfingSnapshot,
    pub(crate) cdl_engulfing_win_loading: bool,
    pub(crate) show_cdl_harami_win: bool,
    pub(crate) cdl_harami_win_symbol: String,
    pub(crate) cdl_harami_win_snapshot: typhoon_engine::core::research::CdlHaramiSnapshot,
    pub(crate) cdl_harami_win_loading: bool,
    // ── CDL* 3-bar / 2-bar patterns ──
    pub(crate) show_cdl_morning_star_win: bool,
    pub(crate) cdl_morning_star_win_symbol: String,
    pub(crate) cdl_morning_star_win_snapshot:
        typhoon_engine::core::research::CdlMorningStarSnapshot,
    pub(crate) cdl_morning_star_win_loading: bool,
    pub(crate) show_cdl_evening_star_win: bool,
    pub(crate) cdl_evening_star_win_symbol: String,
    pub(crate) cdl_evening_star_win_snapshot:
        typhoon_engine::core::research::CdlEveningStarSnapshot,
    pub(crate) cdl_evening_star_win_loading: bool,
    pub(crate) show_cdl_three_black_crows_win: bool,
    pub(crate) cdl_three_black_crows_win_symbol: String,
    pub(crate) cdl_three_black_crows_win_snapshot:
        typhoon_engine::core::research::CdlThreeBlackCrowsSnapshot,
    pub(crate) cdl_three_black_crows_win_loading: bool,
    pub(crate) show_cdl_three_white_soldiers_win: bool,
    pub(crate) cdl_three_white_soldiers_win_symbol: String,
    pub(crate) cdl_three_white_soldiers_win_snapshot:
        typhoon_engine::core::research::CdlThreeWhiteSoldiersSnapshot,
    pub(crate) cdl_three_white_soldiers_win_loading: bool,
    pub(crate) show_cdl_dark_cloud_cover_win: bool,
    pub(crate) cdl_dark_cloud_cover_win_symbol: String,
    pub(crate) cdl_dark_cloud_cover_win_snapshot:
        typhoon_engine::core::research::CdlDarkCloudCoverSnapshot,
    pub(crate) cdl_dark_cloud_cover_win_loading: bool,
    // ── CDL* piercing / doji variants / hammer mirrors ──
    pub(crate) show_cdl_piercing_win: bool,
    pub(crate) cdl_piercing_win_symbol: String,
    pub(crate) cdl_piercing_win_snapshot: typhoon_engine::core::research::CdlPiercingSnapshot,
    pub(crate) cdl_piercing_win_loading: bool,
    pub(crate) show_cdl_dragonfly_doji_win: bool,
    pub(crate) cdl_dragonfly_doji_win_symbol: String,
    pub(crate) cdl_dragonfly_doji_win_snapshot:
        typhoon_engine::core::research::CdlDragonflyDojiSnapshot,
    pub(crate) cdl_dragonfly_doji_win_loading: bool,
    pub(crate) show_cdl_gravestone_doji_win: bool,
    pub(crate) cdl_gravestone_doji_win_symbol: String,
    pub(crate) cdl_gravestone_doji_win_snapshot:
        typhoon_engine::core::research::CdlGravestoneDojiSnapshot,
    pub(crate) cdl_gravestone_doji_win_loading: bool,
    pub(crate) show_cdl_hanging_man_win: bool,
    pub(crate) cdl_hanging_man_win_symbol: String,
    pub(crate) cdl_hanging_man_win_snapshot: typhoon_engine::core::research::CdlHangingManSnapshot,
    pub(crate) cdl_hanging_man_win_loading: bool,
    pub(crate) show_cdl_inverted_hammer_win: bool,
    pub(crate) cdl_inverted_hammer_win_symbol: String,
    pub(crate) cdl_inverted_hammer_win_snapshot:
        typhoon_engine::core::research::CdlInvertedHammerSnapshot,
    pub(crate) cdl_inverted_hammer_win_loading: bool,
    // ── CDL* harami cross / long-legged doji / marubozu / spinning top / tristar ──
    pub(crate) show_cdl_harami_cross_win: bool,
    pub(crate) cdl_harami_cross_win_symbol: String,
    pub(crate) cdl_harami_cross_win_snapshot:
        typhoon_engine::core::research::CdlHaramiCrossSnapshot,
    pub(crate) cdl_harami_cross_win_loading: bool,
    pub(crate) show_cdl_long_legged_doji_win: bool,
    pub(crate) cdl_long_legged_doji_win_symbol: String,
    pub(crate) cdl_long_legged_doji_win_snapshot:
        typhoon_engine::core::research::CdlLongLeggedDojiSnapshot,
    pub(crate) cdl_long_legged_doji_win_loading: bool,
    pub(crate) show_cdl_marubozu_win: bool,
    pub(crate) cdl_marubozu_win_symbol: String,
    pub(crate) cdl_marubozu_win_snapshot: typhoon_engine::core::research::CdlMarubozuSnapshot,
    pub(crate) cdl_marubozu_win_loading: bool,
    pub(crate) show_cdl_spinning_top_win: bool,
    pub(crate) cdl_spinning_top_win_symbol: String,
    pub(crate) cdl_spinning_top_win_snapshot:
        typhoon_engine::core::research::CdlSpinningTopSnapshot,
    pub(crate) cdl_spinning_top_win_loading: bool,
    pub(crate) show_cdl_tristar_win: bool,
    pub(crate) cdl_tristar_win_symbol: String,
    pub(crate) cdl_tristar_win_snapshot: typhoon_engine::core::research::CdlTristarSnapshot,
    pub(crate) cdl_tristar_win_loading: bool,
    // ── CDL* doji star / morning doji star / evening doji star / abandoned baby / three inside ──
    pub(crate) show_cdl_doji_star_win: bool,
    pub(crate) cdl_doji_star_win_symbol: String,
    pub(crate) cdl_doji_star_win_snapshot: typhoon_engine::core::research::CdlDojiStarSnapshot,
    pub(crate) cdl_doji_star_win_loading: bool,
    pub(crate) show_cdl_morning_doji_star_win: bool,
    pub(crate) cdl_morning_doji_star_win_symbol: String,
    pub(crate) cdl_morning_doji_star_win_snapshot:
        typhoon_engine::core::research::CdlMorningDojiStarSnapshot,
    pub(crate) cdl_morning_doji_star_win_loading: bool,
    pub(crate) show_cdl_evening_doji_star_win: bool,
    pub(crate) cdl_evening_doji_star_win_symbol: String,
    pub(crate) cdl_evening_doji_star_win_snapshot:
        typhoon_engine::core::research::CdlEveningDojiStarSnapshot,
    pub(crate) cdl_evening_doji_star_win_loading: bool,
    pub(crate) show_cdl_abandoned_baby_win: bool,
    pub(crate) cdl_abandoned_baby_win_symbol: String,
    pub(crate) cdl_abandoned_baby_win_snapshot:
        typhoon_engine::core::research::CdlAbandonedBabySnapshot,
    pub(crate) cdl_abandoned_baby_win_loading: bool,
    pub(crate) show_cdl_three_inside_win: bool,
    pub(crate) cdl_three_inside_win_symbol: String,
    pub(crate) cdl_three_inside_win_snapshot:
        typhoon_engine::core::research::CdlThreeInsideSnapshot,
    pub(crate) cdl_three_inside_win_loading: bool,
    // ── CDL* belt hold / closing marubozu / high wave / long line / short line ──
    pub(crate) show_cdl_belt_hold_win: bool,
    pub(crate) cdl_belt_hold_win_symbol: String,
    pub(crate) cdl_belt_hold_win_snapshot: typhoon_engine::core::research::CdlBeltHoldSnapshot,
    pub(crate) cdl_belt_hold_win_loading: bool,
    pub(crate) show_cdl_closing_marubozu_win: bool,
    pub(crate) cdl_closing_marubozu_win_symbol: String,
    pub(crate) cdl_closing_marubozu_win_snapshot:
        typhoon_engine::core::research::CdlClosingMarubozuSnapshot,
    pub(crate) cdl_closing_marubozu_win_loading: bool,
    pub(crate) show_cdl_high_wave_win: bool,
    pub(crate) cdl_high_wave_win_symbol: String,
    pub(crate) cdl_high_wave_win_snapshot: typhoon_engine::core::research::CdlHighWaveSnapshot,
    pub(crate) cdl_high_wave_win_loading: bool,
    pub(crate) show_cdl_long_line_win: bool,
    pub(crate) cdl_long_line_win_symbol: String,
    pub(crate) cdl_long_line_win_snapshot: typhoon_engine::core::research::CdlLongLineSnapshot,
    pub(crate) cdl_long_line_win_loading: bool,
    pub(crate) show_cdl_short_line_win: bool,
    pub(crate) cdl_short_line_win_symbol: String,
    pub(crate) cdl_short_line_win_snapshot: typhoon_engine::core::research::CdlShortLineSnapshot,
    pub(crate) cdl_short_line_win_loading: bool,
    // ── CDL* counterattack / homing pigeon / in-neck / on-neck / thrusting ──
    pub(crate) show_cdl_counterattack_win: bool,
    pub(crate) cdl_counterattack_win_symbol: String,
    pub(crate) cdl_counterattack_win_snapshot:
        typhoon_engine::core::research::CdlCounterattackSnapshot,
    pub(crate) cdl_counterattack_win_loading: bool,
    pub(crate) show_cdl_homing_pigeon_win: bool,
    pub(crate) cdl_homing_pigeon_win_symbol: String,
    pub(crate) cdl_homing_pigeon_win_snapshot:
        typhoon_engine::core::research::CdlHomingPigeonSnapshot,
    pub(crate) cdl_homing_pigeon_win_loading: bool,
    pub(crate) show_cdl_in_neck_win: bool,
    pub(crate) cdl_in_neck_win_symbol: String,
    pub(crate) cdl_in_neck_win_snapshot: typhoon_engine::core::research::CdlInNeckSnapshot,
    pub(crate) cdl_in_neck_win_loading: bool,
    pub(crate) show_cdl_on_neck_win: bool,
    pub(crate) cdl_on_neck_win_symbol: String,
    pub(crate) cdl_on_neck_win_snapshot: typhoon_engine::core::research::CdlOnNeckSnapshot,
    pub(crate) cdl_on_neck_win_loading: bool,
    pub(crate) show_cdl_thrusting_win: bool,
    pub(crate) cdl_thrusting_win_symbol: String,
    pub(crate) cdl_thrusting_win_snapshot: typhoon_engine::core::research::CdlThrustingSnapshot,
    pub(crate) cdl_thrusting_win_loading: bool,
    // ── additional CDL* parity windows ──
    pub(crate) show_cdl_two_crows_win: bool,
    pub(crate) cdl_two_crows_win_symbol: String,
    pub(crate) cdl_two_crows_win_snapshot: typhoon_engine::core::research::CdlTwoCrowsSnapshot,
    pub(crate) cdl_two_crows_win_loading: bool,
    pub(crate) show_cdl_three_line_strike_win: bool,
    pub(crate) cdl_three_line_strike_win_symbol: String,
    pub(crate) cdl_three_line_strike_win_snapshot:
        typhoon_engine::core::research::CdlThreeLineStrikeSnapshot,
    pub(crate) cdl_three_line_strike_win_loading: bool,
    pub(crate) show_cdl_three_outside_win: bool,
    pub(crate) cdl_three_outside_win_symbol: String,
    pub(crate) cdl_three_outside_win_snapshot:
        typhoon_engine::core::research::CdlThreeOutsideSnapshot,
    pub(crate) cdl_three_outside_win_loading: bool,
    pub(crate) show_cdl_matching_low_win: bool,
    pub(crate) cdl_matching_low_win_symbol: String,
    pub(crate) cdl_matching_low_win_snapshot:
        typhoon_engine::core::research::CdlMatchingLowSnapshot,
    pub(crate) cdl_matching_low_win_loading: bool,
    pub(crate) show_cdl_separating_lines_win: bool,
    pub(crate) cdl_separating_lines_win_symbol: String,
    pub(crate) cdl_separating_lines_win_snapshot:
        typhoon_engine::core::research::CdlSeparatingLinesSnapshot,
    pub(crate) cdl_separating_lines_win_loading: bool,
    pub(crate) show_cdl_stick_sandwich_win: bool,
    pub(crate) cdl_stick_sandwich_win_symbol: String,
    pub(crate) cdl_stick_sandwich_win_snapshot:
        typhoon_engine::core::research::CdlStickSandwichSnapshot,
    pub(crate) cdl_stick_sandwich_win_loading: bool,
    pub(crate) show_cdl_rickshaw_man_win: bool,
    pub(crate) cdl_rickshaw_man_win_symbol: String,
    pub(crate) cdl_rickshaw_man_win_snapshot:
        typhoon_engine::core::research::CdlRickshawManSnapshot,
    pub(crate) cdl_rickshaw_man_win_loading: bool,
    pub(crate) show_cdl_takuri_win: bool,
    pub(crate) cdl_takuri_win_symbol: String,
    pub(crate) cdl_takuri_win_snapshot: typhoon_engine::core::research::CdlTakuriSnapshot,
    pub(crate) cdl_takuri_win_loading: bool,
    // ── harder CDL* parity windows ──
    pub(crate) show_cdl_three_stars_in_south_win: bool,
    pub(crate) cdl_three_stars_in_south_win_symbol: String,
    pub(crate) cdl_three_stars_in_south_win_snapshot:
        typhoon_engine::core::research::CdlThreeStarsInSouthSnapshot,
    pub(crate) cdl_three_stars_in_south_win_loading: bool,
    pub(crate) show_cdl_identical_three_crows_win: bool,
    pub(crate) cdl_identical_three_crows_win_symbol: String,
    pub(crate) cdl_identical_three_crows_win_snapshot:
        typhoon_engine::core::research::CdlIdenticalThreeCrowsSnapshot,
    pub(crate) cdl_identical_three_crows_win_loading: bool,
    pub(crate) show_cdl_kicking_win: bool,
    pub(crate) cdl_kicking_win_symbol: String,
    pub(crate) cdl_kicking_win_snapshot: typhoon_engine::core::research::CdlKickingSnapshot,
    pub(crate) cdl_kicking_win_loading: bool,
    pub(crate) show_cdl_kicking_by_length_win: bool,
    pub(crate) cdl_kicking_by_length_win_symbol: String,
    pub(crate) cdl_kicking_by_length_win_snapshot:
        typhoon_engine::core::research::CdlKickingByLengthSnapshot,
    pub(crate) cdl_kicking_by_length_win_loading: bool,
    pub(crate) show_cdl_ladder_bottom_win: bool,
    pub(crate) cdl_ladder_bottom_win_symbol: String,
    pub(crate) cdl_ladder_bottom_win_snapshot:
        typhoon_engine::core::research::CdlLadderBottomSnapshot,
    pub(crate) cdl_ladder_bottom_win_loading: bool,
    pub(crate) show_cdl_unique_three_river_win: bool,
    pub(crate) cdl_unique_three_river_win_symbol: String,
    pub(crate) cdl_unique_three_river_win_snapshot:
        typhoon_engine::core::research::CdlUniqueThreeRiverSnapshot,
    pub(crate) cdl_unique_three_river_win_loading: bool,
    // ── additional multi-bar CDL* parity windows ──
    pub(crate) show_cdl_advance_block_win: bool,
    pub(crate) cdl_advance_block_win_symbol: String,
    pub(crate) cdl_advance_block_win_snapshot:
        typhoon_engine::core::research::CdlAdvanceBlockSnapshot,
    pub(crate) cdl_advance_block_win_loading: bool,
    pub(crate) show_cdl_breakaway_win: bool,
    pub(crate) cdl_breakaway_win_symbol: String,
    pub(crate) cdl_breakaway_win_snapshot: typhoon_engine::core::research::CdlBreakawaySnapshot,
    pub(crate) cdl_breakaway_win_loading: bool,
    pub(crate) show_cdl_gap_side_side_white_win: bool,
    pub(crate) cdl_gap_side_side_white_win_symbol: String,
    pub(crate) cdl_gap_side_side_white_win_snapshot:
        typhoon_engine::core::research::CdlGapSideSideWhiteSnapshot,
    pub(crate) cdl_gap_side_side_white_win_loading: bool,
    pub(crate) show_cdl_upside_gap_two_crows_win: bool,
    pub(crate) cdl_upside_gap_two_crows_win_symbol: String,
    pub(crate) cdl_upside_gap_two_crows_win_snapshot:
        typhoon_engine::core::research::CdlUpsideGapTwoCrowsSnapshot,
    pub(crate) cdl_upside_gap_two_crows_win_loading: bool,
    pub(crate) show_cdl_xside_gap_three_methods_win: bool,
    pub(crate) cdl_xside_gap_three_methods_win_symbol: String,
    pub(crate) cdl_xside_gap_three_methods_win_snapshot:
        typhoon_engine::core::research::CdlXSideGapThreeMethodsSnapshot,
    pub(crate) cdl_xside_gap_three_methods_win_loading: bool,
    pub(crate) show_cdl_conceal_baby_swallow_win: bool,
    pub(crate) cdl_conceal_baby_swallow_win_symbol: String,
    pub(crate) cdl_conceal_baby_swallow_win_snapshot:
        typhoon_engine::core::research::CdlConcealBabySwallowSnapshot,
    pub(crate) cdl_conceal_baby_swallow_win_loading: bool,
    // ── stateful CDL* parity windows ──
    pub(crate) show_cdl_hikkake_win: bool,
    pub(crate) cdl_hikkake_win_symbol: String,
    pub(crate) cdl_hikkake_win_snapshot: typhoon_engine::core::research::CdlHikkakeSnapshot,
    pub(crate) cdl_hikkake_win_loading: bool,
    pub(crate) show_cdl_hikkake_mod_win: bool,
    pub(crate) cdl_hikkake_mod_win_symbol: String,
    pub(crate) cdl_hikkake_mod_win_snapshot: typhoon_engine::core::research::CdlHikkakeModSnapshot,
    pub(crate) cdl_hikkake_mod_win_loading: bool,
    pub(crate) show_cdl_mat_hold_win: bool,
    pub(crate) cdl_mat_hold_win_symbol: String,
    pub(crate) cdl_mat_hold_win_snapshot: typhoon_engine::core::research::CdlMatHoldSnapshot,
    pub(crate) cdl_mat_hold_win_loading: bool,
    pub(crate) show_cdl_rise_fall_three_methods_win: bool,
    pub(crate) cdl_rise_fall_three_methods_win_symbol: String,
    pub(crate) cdl_rise_fall_three_methods_win_snapshot:
        typhoon_engine::core::research::CdlRiseFallThreeMethodsSnapshot,
    pub(crate) cdl_rise_fall_three_methods_win_loading: bool,
    // ── final CDL* parity windows ──
    pub(crate) show_cdl_stalled_pattern_win: bool,
    pub(crate) cdl_stalled_pattern_win_symbol: String,
    pub(crate) cdl_stalled_pattern_win_snapshot:
        typhoon_engine::core::research::CdlStalledPatternSnapshot,
    pub(crate) cdl_stalled_pattern_win_loading: bool,
    pub(crate) show_cdl_tasuki_gap_win: bool,
    pub(crate) cdl_tasuki_gap_win_symbol: String,
    pub(crate) cdl_tasuki_gap_win_snapshot: typhoon_engine::core::research::CdlTasukiGapSnapshot,
    pub(crate) cdl_tasuki_gap_win_loading: bool,
    // ── Quant Stats (modsharpe / hsieh / chow / driftburst / hlvclust) ──
    pub(crate) show_modsharpe_win: bool,
    pub(crate) modsharpe_win_symbol: String,
    pub(crate) modsharpe_win_snapshot: typhoon_engine::core::research::ModSharpeSnapshot,
    pub(crate) modsharpe_win_loading: bool,
    pub(crate) show_hsiehtest_win: bool,
    pub(crate) hsiehtest_win_symbol: String,
    pub(crate) hsiehtest_win_snapshot: typhoon_engine::core::research::HsiehTestSnapshot,
    pub(crate) hsiehtest_win_loading: bool,
    pub(crate) show_chowbreak_win: bool,
    pub(crate) chowbreak_win_symbol: String,
    pub(crate) chowbreak_win_snapshot: typhoon_engine::core::research::ChowBreakSnapshot,
    pub(crate) chowbreak_win_loading: bool,
    pub(crate) show_driftburst_win: bool,
    pub(crate) driftburst_win_symbol: String,
    pub(crate) driftburst_win_snapshot: typhoon_engine::core::research::DriftBurstSnapshot,
    pub(crate) driftburst_win_loading: bool,
    pub(crate) show_hlvclust_win: bool,
    pub(crate) hlvclust_win_symbol: String,
    pub(crate) hlvclust_win_snapshot: typhoon_engine::core::research::HlvClustSnapshot,
    pub(crate) hlvclust_win_loading: bool,
    // ── Quant Stats (yangzhang / kuiper / dagostino / baiperron / kupiecpof) ──
    pub(crate) show_yangzhang_win: bool,
    pub(crate) yangzhang_win_symbol: String,
    pub(crate) yangzhang_win_snapshot: typhoon_engine::core::research::YangZhangVolSnapshot,
    pub(crate) yangzhang_win_loading: bool,
    pub(crate) show_kuiper_win: bool,
    pub(crate) kuiper_win_symbol: String,
    pub(crate) kuiper_win_snapshot: typhoon_engine::core::research::KuiperSnapshot,
    pub(crate) kuiper_win_loading: bool,
    pub(crate) show_dagostino_win: bool,
    pub(crate) dagostino_win_symbol: String,
    pub(crate) dagostino_win_snapshot: typhoon_engine::core::research::DagostinoSnapshot,
    pub(crate) dagostino_win_loading: bool,
    pub(crate) show_baiperron_win: bool,
    pub(crate) baiperron_win_symbol: String,
    pub(crate) baiperron_win_snapshot: typhoon_engine::core::research::BaiPerronSnapshot,
    pub(crate) baiperron_win_loading: bool,
    pub(crate) show_kupiecpof_win: bool,
    pub(crate) kupiecpof_win_symbol: String,
    pub(crate) kupiecpof_win_snapshot: typhoon_engine::core::research::KupiecPofSnapshot,
    pub(crate) kupiecpof_win_loading: bool,

    // ── Web article ingestion + packet viewer ──
    /// INGEST_RESEARCH — paste-in window where the user drops an AI
    /// agent reply that contains `===TYPHOON_INGEST===` blocks.
    pub(crate) show_ingest_research: bool,
    pub(crate) ingest_research_text: String,
    pub(crate) ingest_research_agent: String, // default tag applied to records missing an agent field
    pub(crate) ingest_research_status: String, // last status / result summary
    pub(crate) ingest_research_busy: bool,
    /// RESEARCH_PACKET — viewer window with tree nav + scrollable text.
    pub(crate) show_packet_viewer: bool,
    pub(crate) packet_viewer_symbol: String,
    pub(crate) packet_viewer_question: String,
    pub(crate) packet_viewer_text: String, // generated packet markdown
    pub(crate) packet_viewer_tree: Vec<PacketTreeNode>, // parsed H2/H3/H4 headers
    pub(crate) packet_viewer_scroll_target: Option<usize>, // byte offset in text to scroll to
    pub(crate) packet_viewer_selected: Option<usize>,

    /// Bottom panel tab.
    pub(crate) bottom_tab: BottomTab,

    /// Application log — max 500 entries, ring-buffer style.
    pub(crate) log: VecDeque<LogEntry>,
    /// ADR-094: Log level filter dropdown.
    pub(crate) log_filter: LogFilter,

    // ── ADR-094: UX Analytics Features ──────────────────────────────
    /// Active result card (rendered above log, auto-dismissed after 30s).
    pub(crate) result_card: Option<(ResultCard, std::time::Instant)>,
    /// Toast notification stack (top-right overlay).
    pub(crate) toasts: Vec<Toast>,
    /// Command palette context (set by right-click location).
    pub(crate) palette_context: PaletteContext,

    /// Crosshair position in screen coordinates (updated each frame).
    pub(crate) crosshair: Option<egui::Pos2>,

    /// Counter to avoid calling ctx.request_repaint in a tight loop.
    pub(crate) frame_count: u64,

    /// Tab being dragged (for drag-and-drop reordering).
    pub(crate) dragging_tab: Option<usize>,

    /// Last active tab index the tab strip handled — used to detect active-tab
    /// changes (clicking a tab, the + button, NEW_TAB, close adjustments) so the
    /// horizontally-scrollable tab bar can scroll the active tab into view.
    pub(crate) tab_bar_last_active: usize,

    // ── async broker ─────────────────────────────────────────────────────
    /// Tokio runtime handle for spawning async tasks.
    pub(crate) rt_handle: tokio::runtime::Handle,
    /// Send commands to broker task.
    pub(crate) broker_tx: mpsc::UnboundedSender<BrokerCmd>,
    /// Receive results from broker task.
    pub(crate) broker_rx: mpsc::UnboundedReceiver<BrokerMsg>,
    /// Whether broker is connected.
    pub(crate) broker_connected: bool,
    /// Live account info.
    pub(crate) live_account: Option<AccountInfo>,
    /// Live positions.
    pub(crate) live_positions: Vec<PositionInfo>,
    pub(crate) kr_positions: Vec<PositionInfo>,
    pub(crate) kraken_equity_quote_meta: std::collections::BTreeMap<String, KrakenEquityQuoteMeta>,
    /// Position visibility toggles (still synced, just hidden in UI)
    pub(crate) show_alpaca_positions: bool,
    pub(crate) show_kr_positions: bool,
    pub(crate) show_kraken_trade_history: bool,
    pub(crate) show_kraken_open_orders: bool,
    /// Live orders.
    pub(crate) live_orders: Vec<OrderInfo>,

    // ── right panel state (WebKit parity) ─────────────────────────────
    /// Active right panel tab (kept for session compat).
    pub(crate) right_tab: RightTab,
    /// Collapsible right panel sections (all visible, individually expandable).
    pub(crate) right_trading_open: bool,
    pub(crate) right_positions_open: bool,
    pub(crate) right_orders_open: bool,
    pub(crate) right_watchlist_open: bool,
    pub(crate) right_risk_open: bool,
    pub(crate) right_recent_fills_open: bool,
    pub(crate) right_news_open: bool,
    pub(crate) right_mtf_grid_open: bool,
    pub(crate) right_panel_order: Vec<RightPanelSectionId>,
    pub(crate) dragging_right_panel_section: Option<RightPanelSectionId>,
    /// Risk sizing mode dropdown.
    pub(crate) risk_mode: RiskMode,
    pub(crate) order_broker: OrderBroker,
    /// Primary broker (top-bar switch). The primary broker is the order-routing
    /// default and the trusted/reference lane for the equity data merge; every
    /// other enabled broker acts as a sync **assist** lane. Persisted.
    pub(crate) primary_broker: OrderBroker,
    /// SL price input text.
    pub(crate) sl_input: String,
    /// TP price input text.
    pub(crate) tp_input: String,
    /// Standard mode risk % input.
    pub(crate) trade_risk_pct_input: String,
    /// Dynamic mode minimum balance floor.
    pub(crate) trade_min_balance_input: String,
    /// Dynamic mode losses-to-floor input.
    pub(crate) trade_losses_to_min_input: String,
    /// VaR mode risk % of equity.
    pub(crate) trade_var_risk_pct_input: String,
    /// Whether SL checkbox is enabled.
    pub(crate) sl_enabled: bool,
    /// Whether TP checkbox is enabled.
    pub(crate) tp_enabled: bool,
    /// Recent fills (symbol, side, qty, price, time).
    pub(crate) recent_fills: Vec<(String, String, f64, f64, String)>,

    /// Latest background-computed data. Updated by draining bg_rx each frame.
    pub(crate) bg: BgData,
    /// Receiver for background data snapshots.
    pub(crate) bg_rx: std::sync::mpsc::Receiver<BgData>,

    pub(crate) gpu_indicators: Option<gpu_compute::GpuCompute>,
    /// Set true when indicator periods change in the UI; cleared after recompute.
    pub(crate) indicators_dirty: bool,

    // ── Prometheus metrics ───────────────────────────────────────────────
    /// Shared metrics registry (updated periodically, served via HTTP).
    pub(crate) metrics_registry: Option<std::sync::Arc<crate::metrics::MetricsRegistry>>,
    /// App start time for uptime calculation.
    pub(crate) metrics_start: std::time::Instant,
    /// Wall-clock gates for periodic work. These must not be derived from frame_count:
    /// native-refresh rendering can run at 60/144/240Hz, while old code assumed 4fps idle.
    pub(crate) periodic_crypto_last_refresh: std::time::Instant,
    pub(crate) kraken_universe_last_schedule: std::time::Instant,
    /// Wall-clock anchor for the periodic re-evaluation of the WS OHLC spawn
    /// when focus was empty at startup. Once the streamers are up
    /// (`kraken_ws_ohlc_started == true`) this stops being read.
    pub(crate) kraken_ws_ohlc_last_spawn_retry: std::time::Instant,
    /// Wall-clock anchor for the background news-body hydration tick.
    /// Throttle is HYDRATE_INTERVAL_SECS (see `news_ingest`). One in-flight
    /// hydrate at a time is enforced by `news_body_hydrate_in_flight`.
    pub(crate) news_body_last_hydrate: std::time::Instant,
    pub(crate) news_body_hydrate_in_flight: bool,
    /// CommonMark renderer cache for the article-body pane (egui_commonmark
    /// needs a persistent cache so per-frame parse + image-handle lookup
    /// stays cheap). Hero images render via the egui image loader installed
    /// in `new` on construction.
    pub(crate) news_md_cache: egui_commonmark::CommonMarkCache,
    pub(crate) kraken_futures_universe_last_schedule: std::time::Instant,
    pub(crate) session_last_autosave: std::time::Instant,
    pub(crate) metrics_last_update: std::time::Instant,
    /// Last REST `TradesHistory` fetch. The `ownTrades` WebSocket already
    /// streams new trades live (see KrakenLiveTrade handler), so the REST
    /// pull is only needed at connect / reconnect / cold cache. A periodic
    /// dispatch on every KrakenBalances tick (~60 s) was burning a private
    /// REST counter slot and re-rendering the same history; this gate caps
    /// the cadence to KRAKEN_TRADES_REST_REFRESH_SECS.
    pub(crate) kraken_trades_last_fetch: std::time::Instant,
    pub(crate) weekend_crypto_last_sync: std::time::Instant,
    pub(crate) alpaca_rotation_last_sync: std::time::Instant,
    pub(crate) perf_last_report: std::time::Instant,
    pub(crate) perf_slow_frame_count: u32,
    pub(crate) perf_max_update_ms: f64,
    pub(crate) perf_broker_msgs_drained: u32,

    /// Screenshot requested via SCREENSHOT command (triggers ViewportCommand::Screenshot next frame).
    pub(crate) screenshot_requested: bool,
    /// Path to the last saved screenshot (for sharing to Matrix chat).
    pub(crate) last_screenshot_path: Option<std::path::PathBuf>,

    /// Artefact gallery: scanned list of on-disk screenshot files
    /// (path, mtime unix seconds, size bytes), sorted newest-first.
    pub(crate) screenshots_list: Vec<(std::path::PathBuf, i64, u64)>,
    pub(crate) screenshots_sort_col: usize,
    pub(crate) screenshots_sort_asc: bool,
    /// Wall-clock unix ts of last scan_screenshots() call; throttles
    /// redundant directory walks while the gallery window is open.
    pub(crate) screenshots_last_refresh: i64,
    /// Toggle for the Screenshots Gallery window (palette: SCREENSHOTS / GALLERY).
    pub(crate) show_screenshots_gallery: bool,
}
