//! Extracted from app.rs: commands helpers.

use super::*;

// ─── indicator computation ────────────────────────────────────────────────────

pub(crate) struct Command {
    pub(crate) name: &'static str,
    pub(crate) desc: &'static str,
}

/// Lowercased (name, desc) per COMMANDS index — built once, reused every frame
/// by the palette's fuzzy scorer so we don't allocate two Strings per command
/// per keystroke × 60 fps.
pub(crate) static COMMANDS_LOWER: std::sync::LazyLock<Vec<(String, String)>> =
    std::sync::LazyLock::new(|| {
        COMMANDS
            .iter()
            .map(|c| (c.name.to_lowercase(), c.desc.to_lowercase()))
            .collect()
    });

pub(crate) const COMMANDS: &[Command] = &[
    // Core
    Command {
        name: "CONNECT",
        desc: "Connect to broker (Alpaca / Kraken)",
    },
    Command {
        name: "SETTINGS",
        desc: "Application settings",
    },
    Command {
        name: "RELOAD",
        desc: "Reload bars from cache",
    },
    Command {
        name: "QUIT",
        desc: "Exit the application",
    },
    Command {
        name: "QUOTE",
        desc: "Get latest bid/ask quote for current symbol",
    },
    Command {
        name: "CLOCK",
        desc: "Market clock — open/closed status",
    },
    Command {
        name: "FILLS",
        desc: "Recent account fills/activities",
    },
    Command {
        name: "MOVERS",
        desc: "Top market movers (stocks)",
    },
    Command {
        name: "SEARCH",
        desc: "Search symbols by name",
    },
    Command {
        name: "HISTORY",
        desc: "Order history (closed orders)",
    },
    // View
    Command {
        name: "MTF",
        desc: "Toggle multi-timeframe grid",
    },
    Command {
        name: "MTF_2X2",
        desc: "2×2 grid (4 charts)",
    },
    Command {
        name: "MTF_3X3",
        desc: "3×3 grid (9 charts)",
    },
    Command {
        name: "MTF_4X4",
        desc: "4×4 grid (16 charts)",
    },
    Command {
        name: "MTF_4X3",
        desc: "4×3 grid (12 charts)",
    },
    Command {
        name: "INDICATORS",
        desc: "Toggle indicator settings panel",
    },
    Command {
        name: "FULLSCREEN",
        desc: "Toggle fullscreen mode",
    },
    // Trading
    Command {
        name: "OPEN_TRADE",
        desc: "Open order entry panel",
    },
    Command {
        name: "OCO",
        desc: "OCO exit order — one-cancels-other (OCO SELL AAPL 10 200 180)",
    },
    Command {
        name: "EXPORT_CALENDAR",
        desc: "Export event calendar to ICS file",
    },
    Command {
        name: "CLOSE_ALL",
        desc: "Close all open positions",
    },
    Command {
        name: "CLOSE_PARTIAL",
        desc: "Close 50% of largest position",
    },
    Command {
        name: "SET_SL",
        desc: "Click chart to set stop loss",
    },
    Command {
        name: "SET_TP",
        desc: "Click chart to set take profit",
    },
    // Tools
    Command {
        name: "OVERLAP",
        desc: "Symbol overlap / correlation",
    },
    Command {
        name: "BACKTEST",
        desc: "Run backtest on current symbol",
    },
    Command {
        name: "BACKTEST_UNEXPAND",
        desc: "Remove symbol from backtest expand set",
    },
    Command {
        name: "SCREENER",
        desc: "Symbol screener",
    },
    Command {
        name: "SYMBOLS",
        desc: "Symbol explorer (broker hierarchy)",
    },
    Command {
        name: "OPTIMIZER",
        desc: "Strategy parameter optimizer",
    },
    Command {
        name: "RISK_CALC",
        desc: "Position sizing / risk calculator",
    },
    Command {
        name: "COMPOUND",
        desc: "Compound interest calculator",
    },
    Command {
        name: "VAR",
        desc: "VaR multiplier estimator",
    },
    Command {
        name: "MARGIN",
        desc: "Margin monitor",
    },
    // Research
    Command {
        name: "FRED",
        desc: "FRED economic data dashboard (Fed Funds, CPI, GDP, VIX, yields)",
    },
    Command {
        name: "NEWS",
        desc: "Market news & events",
    },
    Command {
        name: "CALENDAR",
        desc: "Economic calendar — FOMC, NFP, CPI, PMI, earnings releases",
    },
    Command {
        name: "SEC",
        desc: "SEC filings (10-K, 10-Q, 8-K)",
    },
    Command {
        name: "INSIDER",
        desc: "Insider trades (Form 4)",
    },
    Command {
        name: "FUNDAMENTALS",
        desc: "Fundamentals viewer (EV, ratios, profile)",
    },
    Command {
        name: "SCOPE",
        desc: "Set broker scope filter: SCOPE [ALL|ALPACA|KRAKEN|POSITIONS]",
    },
    Command {
        name: "EV",
        desc: "Enterprise Value scanner (all symbols)",
    },
    Command {
        name: "EARNINGS",
        desc: "Upcoming earnings calendar",
    },
    Command {
        name: "DIVIDENDS",
        desc: "Upcoming dividend calendar",
    },
    Command {
        name: "EVSCRAPE",
        desc: "Scrape fundamentals for all watchlist symbols (EVSCRAPE FORCE to bypass 24h cache)",
    },
    Command {
        name: "ANALYST",
        desc: "Analyst ratings, price targets & recommendations",
    },
    Command {
        name: "SHORT_INTEREST",
        desc: "Short interest data (Finnhub)",
    },
    Command {
        name: "CORPORATE",
        desc: "Corporate actions (splits, dividends, mergers)",
    },
    Command {
        name: "MOST_ACTIVE",
        desc: "Most active stocks by volume",
    },
    Command {
        name: "PORTFOLIO_HIST",
        desc: "Portfolio equity history",
    },
    Command {
        name: "HOLDERS",
        desc: "Institutional holders",
    },
    Command {
        name: "OPTIONS",
        desc: "Options chain for current symbol (Alpaca + tastytrade)",
    },
    Command {
        name: "WATCHLISTS",
        desc: "Alpaca watchlists",
    },
    Command {
        name: "COMPILE",
        desc: "Indicator compiler: MQL5/MQL4/PineScript v4+v5/EasyLanguage/thinkScript/AFL/ProBuilder/NinjaScript/cAlgo + cross-language transpiler",
    },
    Command {
        name: "STREAM",
        desc: "Start real-time WebSocket stream for current symbol",
    },
    Command {
        name: "DXLINK_STREAM",
        desc: "Start DXLink real-time quote stream (tastytrade)",
    },
    // Analysis
    Command {
        name: "CORRELATION",
        desc: "Correlation matrix",
    },
    Command {
        name: "SEASONALS",
        desc: "Seasonal patterns",
    },
    Command {
        name: "MONTECARLO",
        desc: "Monte Carlo VaR simulation",
    },
    Command {
        name: "STRESS_TEST",
        desc: "Portfolio stress test",
    },
    Command {
        name: "VOLUME_PROFILE",
        desc: "Volume profile (POC + value area)",
    },
    Command {
        name: "ORDER_FLOW",
        desc: "Order flow / delta analysis",
    },
    Command {
        name: "BOOKMAP",
        desc: "Bookmap depth heatmap; use BOOKMAP SYMBOL to open a specific symbol",
    },
    Command {
        name: "HV_CONE",
        desc: "Historical volatility cone (percentile rank)",
    },
    Command {
        name: "SECTOR_HEATMAP",
        desc: "Sector performance heatmap",
    },
    Command {
        name: "DIVSCREEN",
        desc: "Dividend yield screener (ranked by yield)",
    },
    Command {
        name: "EVENTS",
        desc: "Upcoming events calendar (earnings/dividends) per broker",
    },
    Command {
        name: "CONFLUENCE",
        desc: "Multi-timeframe RSI/MACD confluence score",
    },
    Command {
        name: "STAT_ARB",
        desc: "Statistical arbitrage pairs (z-score + half-life)",
    },
    Command {
        name: "RISK_BUDGET",
        desc: "Portfolio risk budget (marginal VaR contribution)",
    },
    // Chart types
    Command {
        name: "CANDLE",
        desc: "Switch to candlestick chart",
    },
    Command {
        name: "HEIKINASHI",
        desc: "Switch to Heikin-Ashi chart",
    },
    Command {
        name: "LINE",
        desc: "Switch to line chart",
    },
    Command {
        name: "OHLC",
        desc: "Switch to OHLC bars chart",
    },
    Command {
        name: "RENKO",
        desc: "Switch to Renko chart",
    },
    Command {
        name: "EXPORT_CSV",
        desc: "Export chart data to CSV",
    },
    Command {
        name: "SCREENSHOT",
        desc: "Save chart as lossless WebP screenshot",
    },
    Command {
        name: "SHARE",
        desc: "Share last screenshot to community chat",
    },
    Command {
        name: "NEW_TAB",
        desc: "Open new chart tab",
    },
    Command {
        name: "CLOSE_TAB",
        desc: "Close current chart tab",
    },
    Command {
        name: "OUTLIERS",
        desc: "Multi-dim outlier scanner: P/E + EV + short ratio + SEC filings",
    },
    Command {
        name: "EVOUTLIERS",
        desc: "Outlier scanner on enterprise value (EV), grouped by sector",
    },
    Command {
        name: "VAROUTLIER",
        desc: "VaR/Ask ratio IQR analysis per sector/industry",
    },
    Command {
        name: "ATROUTLIER",
        desc: "ATR/Price ratio IQR outlier detection per sector",
    },
    Command {
        name: "SCRAPESTATUS",
        desc: "Scrape status dashboard — fundamentals, SEC, crypto",
    },
    Command {
        name: "WEBSERVER",
        desc: "Start HTTPS web server for phone access over LAN",
    },
    Command {
        name: "ALERTS",
        desc: "Indicator alert builder (RSI, MACD, Fisher, Price conditions)",
    },
    Command {
        name: "RISKRUIN",
        desc: "Risk-of-Ruin calculator (Monte Carlo equity path simulation)",
    },
    Command {
        name: "REPLAY",
        desc: "Market replay mode — step through history bar-by-bar",
    },
    Command {
        name: "STORAGE",
        desc: "Cache storage manager — view and delete data by symbol/source",
    },
    Command {
        name: "SYNC",
        desc: "Bar sync status — % healthy per broker/TF across Kraken, Alpaca",
    },
    // Drawing tools
    Command {
        name: "DRAW_HLINE",
        desc: "Draw horizontal line",
    },
    Command {
        name: "DRAW_TRENDLINE",
        desc: "Draw trendline (2 clicks)",
    },
    Command {
        name: "DRAW_FIBO",
        desc: "Draw Fibonacci retracement",
    },
    Command {
        name: "JOURNAL",
        desc: "Trade Journal — log trades with notes",
    },
    Command {
        name: "DRAW_VLINE",
        desc: "Draw vertical line",
    },
    Command {
        name: "DRAW_RECT",
        desc: "Draw rectangle zone",
    },
    Command {
        name: "DRAW_RAY",
        desc: "Draw ray (extends right)",
    },
    Command {
        name: "DRAW_CHANNEL",
        desc: "Draw parallel channel (3 clicks)",
    },
    Command {
        name: "DRAW_PARALLEL_CH",
        desc: "Draw parallel channel (2 clicks)",
    },
    Command {
        name: "DRAW_FIB_CHANNEL",
        desc: "Draw Fibonacci channel (3 clicks)",
    },
    Command {
        name: "DRAW_FIB_TIME",
        desc: "Draw Fibonacci time zones",
    },
    Command {
        name: "DRAW_PRICE_LABEL",
        desc: "Draw price label with badge",
    },
    Command {
        name: "DRAW_CALLOUT",
        desc: "Draw callout with arrow (2 clicks)",
    },
    Command {
        name: "DRAW_HIGHLIGHTER",
        desc: "Draw highlighter zone (2 clicks)",
    },
    Command {
        name: "DRAW_CROSS_MARKER",
        desc: "Draw cross marker (+)",
    },
    Command {
        name: "DRAW_POLYLINE",
        desc: "Draw polyline (multi-click, dbl-click end)",
    },
    Command {
        name: "DRAW_ANCHOR_NOTE",
        desc: "Draw anchor note with text box",
    },
    Command {
        name: "DRAW_REGRESSION",
        desc: "Draw regression channel with StdDev bands",
    },
    Command {
        name: "DRAW_GANN_BOX",
        desc: "Draw Gann Box with grid lines (2 clicks)",
    },
    Command {
        name: "DRAW_ELLIOTT",
        desc: "Draw Elliott Wave labels 1-5 (5 clicks)",
    },
    Command {
        name: "DRAW_ABC",
        desc: "Draw ABC correction labels (3 clicks)",
    },
    Command {
        name: "DRAW_DATE_RANGE",
        desc: "Draw date range measurement (2 clicks)",
    },
    Command {
        name: "DRAW_DATE_PRICE",
        desc: "Draw date & price range measurement (2 clicks)",
    },
    Command {
        name: "DRAW_HEAD_SHOULDERS",
        desc: "Draw Head & Shoulders pattern (5 clicks)",
    },
    Command {
        name: "DRAW_XABCD",
        desc: "Draw XABCD harmonic pattern (5 clicks)",
    },
    Command {
        name: "DRAW_BRUSH",
        desc: "Draw freehand brush (click-drag)",
    },
    Command {
        name: "DRAW_FIB_CIRCLE",
        desc: "Draw Fibonacci circle (2 clicks: center + radius)",
    },
    Command {
        name: "DRAW_ARC",
        desc: "Draw arc through 3 points",
    },
    Command {
        name: "DRAW_CURVE",
        desc: "Draw Bezier curve (4 clicks: start, ctrl1, ctrl2, end)",
    },
    Command {
        name: "DRAW_PATH",
        desc: "Draw smooth path (multi-click, dbl-click end)",
    },
    Command {
        name: "DRAW_FORECAST",
        desc: "Draw forecast projection (2 clicks)",
    },
    Command {
        name: "DRAW_GHOST_FEED",
        desc: "Draw ghost feed (mirrors history forward, 2 clicks)",
    },
    Command {
        name: "DRAW_SIGNPOST",
        desc: "Draw signpost marker",
    },
    Command {
        name: "DRAW_RULER",
        desc: "Draw ruler (price, bars, % measurement)",
    },
    Command {
        name: "DRAW_TIME_CYCLE",
        desc: "Draw time cycle with semi-circles (2 clicks)",
    },
    Command {
        name: "DRAW_SPEED_FAN",
        desc: "Draw speed resistance fan (3 clicks)",
    },
    Command {
        name: "DRAW_SPEED_ARC",
        desc: "Draw speed resistance arc (3 clicks)",
    },
    Command {
        name: "DRAW_FIB_SPIRAL",
        desc: "Draw Fibonacci spiral (2 clicks: center + radius)",
    },
    Command {
        name: "DRAW_ROTATED_RECT",
        desc: "Draw rotated rectangle (3 clicks)",
    },
    Command {
        name: "DRAW_ANCHORED_VWAP",
        desc: "Draw anchored VWAP line from bar",
    },
    Command {
        name: "DRAW_ANCHORED_TEXT",
        desc: "Draw anchored text (1-click)",
    },
    Command {
        name: "DRAW_COMMENT",
        desc: "Draw comment note (1-click)",
    },
    Command {
        name: "DRAW_ARROW_LEFT",
        desc: "Draw left arrow marker (1-click)",
    },
    Command {
        name: "DRAW_ARROW_RIGHT",
        desc: "Draw right arrow marker (1-click)",
    },
    Command {
        name: "DRAW_CIRCLE",
        desc: "Draw circle (2-click center + radius)",
    },
    Command {
        name: "DRAW_PITCH_FAN",
        desc: "Draw pitch fan (2 clicks)",
    },
    Command {
        name: "DRAW_TREND_FIB_TIME",
        desc: "Draw trend-based fib time (2 clicks)",
    },
    Command {
        name: "DRAW_GANN_SQUARE",
        desc: "Draw Gann square (2 clicks)",
    },
    Command {
        name: "DRAW_GANN_SQUARE_FIXED",
        desc: "Draw Gann square fixed (2 clicks)",
    },
    Command {
        name: "DRAW_BARS_PATTERN",
        desc: "Draw bars pattern (2 clicks)",
    },
    Command {
        name: "DRAW_PROJECTION",
        desc: "Draw projection (2 clicks)",
    },
    Command {
        name: "DRAW_DOUBLE_CURVE",
        desc: "Draw double curve (2 clicks)",
    },
    Command {
        name: "DRAW_TRIANGLE_PATTERN",
        desc: "Draw triangle pattern (3 clicks)",
    },
    Command {
        name: "DRAW_THREE_DRIVES",
        desc: "Draw three drives pattern (3 clicks)",
    },
    Command {
        name: "DRAW_ELLIOTT_DOUBLE",
        desc: "Draw Elliott double combo WXY (3 clicks)",
    },
    Command {
        name: "DRAW_ABCD",
        desc: "Draw ABCD pattern (4 clicks)",
    },
    Command {
        name: "DRAW_CYPHER",
        desc: "Draw Cypher pattern (5 clicks)",
    },
    Command {
        name: "DRAW_ELLIOTT_TRIANGLE",
        desc: "Draw Elliott triangle ABCDE (5 clicks)",
    },
    Command {
        name: "DRAW_ELLIOTT_TRIPLE",
        desc: "Draw Elliott triple combo WXYXZ (5 clicks)",
    },
    Command {
        name: "DRAW_ERASER",
        desc: "Eraser mode — click to delete drawings",
    },
    Command {
        name: "CLEAR_DRAWINGS",
        desc: "Clear all drawings on chart",
    },
    Command {
        name: "SESSIONS",
        desc: "Toggle trading session highlighting (Asian/London/NY)",
    },
    Command {
        name: "VOL_HEATMAP",
        desc: "Toggle volume heatmap candle coloring",
    },
    Command {
        name: "VWAP",
        desc: "Toggle VWAP with deviation bands",
    },
    Command {
        name: "PRICE_HIST",
        desc: "Toggle price distribution histogram",
    },
    Command {
        name: "SUPERTREND",
        desc: "Toggle Supertrend indicator (ATR-based trend)",
    },
    Command {
        name: "DONCHIAN",
        desc: "Toggle Donchian Channels (N-bar high/low)",
    },
    Command {
        name: "KELTNER",
        desc: "Toggle Keltner Channels (EMA ± ATR)",
    },
    Command {
        name: "REGRESSION",
        desc: "Toggle Regression Channel (linear regression ± 2σ)",
    },
    Command {
        name: "SQUEEZE",
        desc: "Toggle Squeeze Momentum (BB inside KC)",
    },
    Command {
        name: "VAROSC",
        desc: "Toggle VaR Oscillator (20-bar rolling 95% VaR units)",
    },
    Command {
        name: "CMO_CHART",
        desc: "Toggle chart CMO pane (period 9)",
    },
    Command {
        name: "QSTICK_CHART",
        desc: "Toggle chart QStick pane (period 14)",
    },
    Command {
        name: "DISPARITY_CHART",
        desc: "Toggle chart Disparity pane (period 14)",
    },
    Command {
        name: "BOP_CHART",
        desc: "Toggle chart BOP pane (period 14)",
    },
    Command {
        name: "STDDEV_CHART",
        desc: "Toggle chart StdDev pane (period 20)",
    },
    Command {
        name: "MFI_CHART",
        desc: "Toggle chart MFI pane (period 14)",
    },
    Command {
        name: "TRIX_CHART",
        desc: "Toggle chart TRIX pane (15,9)",
    },
    Command {
        name: "PPO_CHART",
        desc: "Toggle chart PPO pane (12,26,9)",
    },
    Command {
        name: "ULTOSC_CHART",
        desc: "Toggle chart Ultimate Oscillator pane (7,14,28)",
    },
    Command {
        name: "STOCHRSI_CHART",
        desc: "Toggle chart StochRSI pane (14,14,3,3)",
    },
    Command {
        name: "FVG",
        desc: "Toggle Fair Value Gaps (3-bar imbalance zones)",
    },
    Command {
        name: "ORDER_BLOCKS",
        desc: "Toggle Order Blocks (ICT/Smart Money last opposite candle)",
    },
    Command {
        name: "COPY_CHART",
        desc: "Copy visible chart bars to clipboard as CSV",
    },
    Command {
        name: "OBJECTS",
        desc: "Open drawing object list (manage/delete drawings)",
    },
    // Timeframes (direct switch)
    Command {
        name: "M1",
        desc: "Switch to 1-minute timeframe",
    },
    Command {
        name: "M5",
        desc: "Switch to 5-minute timeframe",
    },
    Command {
        name: "M15",
        desc: "Switch to 15-minute timeframe",
    },
    Command {
        name: "M30",
        desc: "Switch to 30-minute timeframe",
    },
    Command {
        name: "H1",
        desc: "Switch to 1-hour timeframe",
    },
    Command {
        name: "H4",
        desc: "Switch to 4-hour timeframe",
    },
    Command {
        name: "D1",
        desc: "Switch to daily timeframe",
    },
    Command {
        name: "W1",
        desc: "Switch to weekly timeframe",
    },
    Command {
        name: "MN1",
        desc: "Switch to monthly timeframe",
    },
    // Analytics (from old app)
    Command {
        name: "EQUITY",
        desc: "Account equity curve",
    },
    Command {
        name: "TRADESTATS",
        desc: "Trade statistics (win rate, expectancy)",
    },
    Command {
        name: "COMPARE",
        desc: "Normalized multi-symbol overlay",
    },
    Command {
        name: "SPREAD",
        desc: "Price ratio / spread chart",
    },
    Command {
        name: "PIVOTS",
        desc: "Classic pivot points on chart",
    },
    Command {
        name: "HEATMAP",
        desc: "Daily P&L heatmap",
    },
    Command {
        name: "PROFILE",
        desc: "Trading profile (best symbols, times)",
    },
    Command {
        name: "SIGNAL",
        desc: "Composite 0-100 trading signal",
    },
    Command {
        name: "STATUS",
        desc: "Cache, memory, uptime status",
    },
    Command {
        name: "SOURCES",
        desc: "Data source priority, health, and per-symbol overrides",
    },
    // Crypto-specific
    Command {
        name: "KRAKEN_FUTURES",
        desc: "Load Kraken Futures public instrument universe",
    },
    // Data management
    Command {
        name: "BACKUP",
        desc: "Backup settings and cache",
    },
    Command {
        name: "WORKSPACE",
        desc: "Save/restore workspace layout",
    },
    // Misc
    Command {
        name: "CACHE_STATS",
        desc: "Show cache statistics",
    },
    Command {
        name: "CLOSE_WINDOWS",
        desc: "Close all floating windows",
    },
    Command {
        name: "HELP",
        desc: "Keyboard shortcuts reference",
    },
    // NNFX system presets
    Command {
        name: "NNFX",
        desc: "Enable NNFX indicator preset (KAMA+Fisher+ATR+BVol)",
    },
    Command {
        name: "RESET_IND",
        desc: "Disable all indicators",
    },
    // Additional analytics
    Command {
        name: "DATA_WINDOW",
        desc: "All indicator values at cursor",
    },
    // ORDER command removed — use Trading tab Open Trade button
    Command {
        name: "PREV_LEVELS",
        desc: "Toggle previous candle levels (D/W)",
    },
    Command {
        name: "FRACTALS",
        desc: "Toggle Bill Williams fractals",
    },
    Command {
        name: "HARMONICS",
        desc: "Toggle harmonic pattern detection (Carney)",
    },
    Command {
        name: "AUTO_FIB",
        desc: "Auto Fibonacci (fractal swing retracement + extension)",
    },
    Command {
        name: "SUPPLY_DEMAND",
        desc: "Toggle supply/demand zone detection",
    },
    Command {
        name: "LAN_SYNC",
        desc: "LAN sync — start server or connect to server by IP",
    },
    Command {
        name: "NEW_WINDOW",
        desc: "Open new terminal window (separate process, multi-monitor)",
    },
    // Unusual Whales / Godel Terminal features
    Command {
        name: "UNUSUAL_VOLUME",
        desc: "Unusual volume scanner — symbols with volume > 2x 20-day average",
    },
    Command {
        name: "SECTOR_ROTATION",
        desc: "Sector ETF relative performance (11 SPDR sectors)",
    },
    Command {
        name: "CONGRESS",
        desc: "Congressional stock trades (House Stock Watcher)",
    },
    // Chart templates
    Command {
        name: "SAVE_TEMPLATE",
        desc: "Save current indicators as named template (SAVE_TEMPLATE <name>)",
    },
    Command {
        name: "LOAD_TEMPLATE",
        desc: "Load a saved indicator template (LOAD_TEMPLATE <name>)",
    },
    Command {
        name: "TEMPLATES",
        desc: "List all available chart templates",
    },
    Command {
        name: "WORKSPACE_SAVE",
        desc: "Save current window layout as named workspace (WORKSPACE_SAVE <name>)",
    },
    Command {
        name: "WORKSPACE_LOAD",
        desc: "Restore a named workspace layout (WORKSPACE_LOAD <name>)",
    },
    Command {
        name: "WORKSPACES",
        desc: "List all saved workspace presets",
    },
    Command {
        name: "CRYPTO_FEAR_GREED",
        desc: "Crypto Fear & Greed Index (alternative.me)",
    },
    Command {
        name: "ASKAI",
        desc: "Ask AI with full TyphooN data packet — ASKAI SYM[,SYM] [question] (uses current AI provider)",
    },
    Command {
        name: "ASKCLAUDE",
        desc: "Ask Claude Code CLI with full TyphooN data packet — ASKCLAUDE SYM[,SYM] [question]",
    },
    Command {
        name: "ASKGEMINI",
        desc: "Ask Gemini CLI with full TyphooN data packet — ASKGEMINI SYM[,SYM] [question] (uses current Gemini CLI model; arbitrary IDs allowed)",
    },
    Command {
        name: "ASKCODEX",
        desc: "Ask OpenAI Codex CLI with full TyphooN data packet — ASKCODEX SYM[,SYM] [question]",
    },
    Command {
        name: "ASKGROK",
        desc: "Ask Grok Build CLI with full TyphooN data packet — ASKGROK SYM[,SYM] [question]",
    },
    Command {
        name: "AICACHE",
        desc: "Cross-client AI response cache stats",
    },
    Command {
        name: "CHAT",
        desc: "TyphooN Terminal community chat",
    },
    Command {
        name: "WSB",
        desc: "Reddit WallStreetBets hot posts",
    },
    Command {
        name: "BARDATA",
        desc: "Download bar data for all known symbols from all brokers",
    },
    Command {
        name: "INDICES",
        desc: "World stock indices dashboard",
    },
    Command {
        name: "CRYPTO50",
        desc: "Top 50 cryptocurrencies by market cap",
    },
    Command {
        name: "FOREX",
        desc: "Forex major pairs dashboard",
    },
    Command {
        name: "KRAKEN",
        desc: "Kraken crypto exchange (connect, balance, trade)",
    },
    // ADR-092: UX improvements
    Command {
        name: "COMPACT",
        desc: "Toggle compact execution mode (hide indicators + sub-panes)",
    },
    Command {
        name: "RULER",
        desc: "Ruler tool — measure price/time distance",
    },
];

#[cfg(test)]
pub(crate) fn fuzzy_match(query: &str, target: &str) -> bool {
    fuzzy_score(&query.to_lowercase(), &target.to_lowercase()).is_some()
}

/// UX3: Deferred action from a symbol right-click context menu.
#[derive(Debug, Clone)]
pub(crate) enum SymbolAction {
    None,
    OpenChart(String),
    AddWatchlist(String),
    ShowFundamentals,
    ShowSec(String),
    ShowInsider,
}

/// UX7: Draw an inline sparkline from a series of closes.
/// Width × height pixels, color based on first→last delta (green up, red down).
pub(crate) fn draw_inline_sparkline(
    ui: &mut egui::Ui,
    closes: &[f64],
    width: f32,
    height: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    if closes.len() < 2 {
        return resp;
    }
    let min = closes.iter().copied().fold(f64::INFINITY, f64::min);
    let max = closes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(1e-9);
    let n = closes.len();
    let color = if closes[n - 1] >= closes[0] {
        egui::Color32::from_rgb(46, 204, 113)
    } else {
        egui::Color32::from_rgb(231, 76, 60)
    };
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(n);
    for (i, &c) in closes.iter().enumerate() {
        let x = rect.left() + (i as f32 / (n - 1) as f32) * width;
        let y = rect.bottom() - ((c - min) / range) as f32 * height;
        points.push(egui::pos2(x, y));
    }
    ui.painter()
        .add(egui::Shape::line(points, egui::Stroke::new(1.0, color)));
    resp
}

/// UX3: Render a label with right-click context menu, returns deferred action.
/// Free function — no &mut self conflict with egui::Window borrows.
pub(crate) fn symbol_label_with_menu(
    ui: &mut egui::Ui,
    symbol: &str,
    label: egui::RichText,
) -> (egui::Response, SymbolAction) {
    let resp = ui.label(label);
    let mut action = SymbolAction::None;
    let sym = symbol.to_string();
    resp.clone().context_menu(|ui| {
        if ui.button("Open chart").clicked() {
            action = SymbolAction::OpenChart(sym.clone());
            ui.close();
        }
        if ui.button("Add to watchlist").clicked() {
            action = SymbolAction::AddWatchlist(sym.clone());
            ui.close();
        }
        if ui.button("View fundamentals").clicked() {
            action = SymbolAction::ShowFundamentals;
            ui.close();
        }
        if ui.button("View SEC filings").clicked() {
            action = SymbolAction::ShowSec(sym.clone());
            ui.close();
        }
        if ui.button("View insider trades").clicked() {
            action = SymbolAction::ShowInsider;
            ui.close();
        }
    });
    (resp, action)
}

/// Fuzzy subsequence match with score (lower = better, None = no match).
/// "voutl" matches "VAROUTLIER" (subsequence), "SEC" matches "SEC Filings" (prefix bonus).
/// Expects both `q` and `t` to be **already lowercased** — caller pays the allocation once.
pub(crate) fn fuzzy_score(q: &str, t: &str) -> Option<i32> {
    if q.is_empty() {
        return Some(1000);
    }
    if let Some(pos) = t.find(q) {
        return Some(pos as i32);
    }
    let mut score: i32 = 1000;
    let mut q_iter = q.chars().peekable();
    let mut last_match: i32 = -1;
    for (i, tc) in t.chars().enumerate() {
        if let Some(&qc) = q_iter.peek() {
            if tc == qc {
                q_iter.next();
                if last_match >= 0 {
                    score += (i as i32 - last_match - 1).max(0);
                }
                last_match = i as i32;
            }
        }
    }
    if q_iter.peek().is_none() {
        Some(score)
    } else {
        None
    }
}
