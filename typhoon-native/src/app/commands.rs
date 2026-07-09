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

/// O(1) lookup by command name for recent-command palette ordering.
pub(crate) static COMMANDS_BY_NAME: std::sync::LazyLock<
    std::collections::HashMap<&'static str, &'static Command>,
> = std::sync::LazyLock::new(|| COMMANDS.iter().map(|c| (c.name, c)).collect());

pub(crate) const COMMANDS: &[Command] = &[
    // Core
    Command {
        name: "SETTINGS",
        desc: "Application settings",
    },
    Command {
        name: "TRADECOPY",
        desc: "Copy positions / mirror orders between broker accounts (ADR-130)",
    },
    Command {
        name: "MARKET_MAP",
        desc: "Finviz-style market treemap + sector groups (ADR-116)",
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
    // Tools
    Command {
        name: "BACKTEST",
        desc: "Run backtest on current symbol",
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
        name: "SMA_INTELLIGENCE",
        desc: "SMA Outfit research window — correlated SMA outfits per Unfair Market",
    },
    Command {
        name: "REG_SHO",
        desc: "Reg SHO threshold list — naked-short / fail-to-deliver watch",
    },
    Command {
        name: "HALTS",
        desc: "Trading halts / LULD volatility pauses (live from cache)",
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
        desc: "Options chain for current symbol (Alpaca)",
    },
    Command {
        name: "WATCHLISTS",
        desc: "Alpaca watchlists",
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
        name: "STAT_ARB",
        desc: "Statistical arbitrage pairs (z-score + half-life)",
    },
    Command {
        name: "RISK_BUDGET",
        desc: "Portfolio risk budget (marginal VaR contribution)",
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
        name: "RISKRUIN",
        desc: "Risk-of-Ruin calculator (Monte Carlo equity path simulation)",
    },
    Command {
        name: "STORAGE",
        desc: "Cache storage manager — view and delete data by symbol/source",
    },
    Command {
        name: "SYNC",
        desc: "Bar sync status — % healthy per broker/TF across Kraken, Alpaca",
    },
    Command {
        name: "JOURNAL",
        desc: "Trade Journal — log trades with notes",
    },
    // Analytics
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
    // Additional analytics
    // ORDER command removed — use Trading tab Open Trade button
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
        name: "ASKANTIGRAVITY",
        desc: "Ask Antigravity CLI with full TyphooN data packet — ASKANTIGRAVITY SYM[,SYM] [question] (uses current Antigravity CLI model; arbitrary IDs allowed)",
    },
    Command {
        name: "ASKGEMINI",
        desc: "Legacy alias for ASKANTIGRAVITY",
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
        name: "EXPORT_PACKET",
        desc: "Export the TyphooN research packet to a Markdown file — EXPORT_PACKET SYM[,SYM] [question] (Save-As dialog, no AI dispatch)",
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
    /// Open (or focus) a chart for the symbol at a specific timeframe — used by
    /// the Reg SHO window's D1 / W1 quick buttons, where the target TF matters.
    OpenChartTf(String, Timeframe),
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
