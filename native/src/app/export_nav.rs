use super::*;

impl TyphooNApp {
    pub(super) fn export_csv(&mut self) {
        if let Some(chart) = self.charts.get(self.active_tab) {
            if chart.bars.is_empty() {
                self.log.push_back(LogEntry::warn("No bars to export"));
                return;
            }
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("CSV", &["csv"])
                .set_file_name(&format!("{}_{}.csv", chart.symbol, chart.timeframe.label()))
                .set_title("Export Chart Data")
                .save_file()
            {
                match std::fs::File::create(&path) {
                    Ok(mut f) => {
                        let _ = writeln!(f, "timestamp,open,high,low,close,volume");
                        for bar in &chart.bars {
                            let _ = writeln!(
                                f,
                                "{},{},{},{},{},{}",
                                bar.ts_ms, bar.open, bar.high, bar.low, bar.close, bar.volume
                            );
                        }
                        self.log.push_back(LogEntry::info(format!(
                            "Exported {} bars to {}",
                            chart.bars.len(),
                            path.display()
                        )));
                    }
                    Err(e) => {
                        self.log
                            .push_back(LogEntry::err(format!("Export failed: {}", e)));
                    }
                }
            }
        }
    }

    pub(super) fn close_all_windows(&mut self) {
        self.show_settings = false;
        self.show_darwin_accounts = false;
        self.show_darwin_portfolio = false;
        self.show_risk_calc = false;
        self.show_compound_calc = false;
        self.show_ai_chat = false;
        self.show_claude_code = false;
        self.show_gemini_cli = false;
        self.show_codex_cli = false;
        self.show_hermes_cli = false;
        self.show_matrix_chat = false;
        self.show_reddit = false;
        self.show_bardata = false;
        self.show_backtest = false;
        self.show_screener = false;
        self.show_symbols = false;
        self.show_optimizer = false;
        self.show_news = false;
        self.show_calendar = false;
        self.show_sec = false;
        self.show_insider = false;
        self.show_fundamentals = false;
        self.show_analyst = false;
        self.show_holders = false;
        self.show_orderbook_window = false;
        self.show_symbol_overlap = false;
        self.show_correlation = false;
        self.show_seasonals = false;
        self.show_montecarlo = false;
        self.show_stress_test = false;
        self.show_volume_profile = false;
        self.show_order_flow = false;
        self.show_bookmap = false;
        self.bookmap_windows.clear();
        self.show_darwinex_outliers = false;
        self.show_option_chain = false;
        self.show_indicator_compiler = false;
        self.show_risk_ruin = false;
        self.show_alert_builder = false;
        self.show_journal = false;
        self.show_var_mult = false;
        self.show_margin_monitor = false;
        self.show_cache_stats = false;
        self.show_storage = false;
        self.show_sync_status = false;
        self.show_lan_sync = false;
        self.show_help = false;
        self.show_connect = false;
        self.show_indicators_panel = false;
        self.show_data_window = false;
        self.show_alerts = false;
        self.show_swap_harvest = false;
        self.show_darwinex_radar = false;
        self.show_scrape_status = false;
        self.show_darwin_browser = false;
        self.show_ev_scanner = false;
        self.show_earnings_calendar = false;
        self.show_dividend_calendar = false;
        self.show_unusual_volume = false;
        self.show_sector_rotation = false;
        self.show_fred = false;
        self.show_econ_calendar = false;
        self.show_congress = false;
        self.show_fear_greed = false;
        self.show_world_indices = false;
        self.show_crypto_top50 = false;
        self.show_forex_matrix = false;
    }

    // ── chart interaction (zoom / pan) ───────────────────────────────────────

    pub(super) fn handle_zoom(chart: &mut ChartState, delta: f32) {
        if chart.bars.is_empty() {
            return;
        }
        // TradingView-style zoom: scroll up = zoom in (fewer bars), scroll down = zoom out
        // Progressive factor: ~5% per notch (15px), capped at 15% per frame
        let pct = (delta * 0.003).clamp(-0.15, 0.15);
        let factor = 1.0 - pct;
        let old_vis = chart.visible_bars;
        let new_vis = ((old_vis as f32 * factor) as usize).clamp(10, chart.bars.len().max(10));
        // TradingView zoom: anchor to the RIGHT (latest bars stay fixed)
        // view_offset is the rightmost visible bar index
        // When zooming in (fewer bars): offset stays same, we see fewer bars on the left
        // When zooming out (more bars): offset stays same, we see more bars on the left
        let max_off = chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
        // Keep the right edge (view_offset) fixed — just change visible_bars
        chart.view_offset = chart.view_offset.min(max_off);
        chart.visible_bars = new_vis;
    }

    pub(super) fn handle_pan_h(chart: &mut ChartState, dx: f32, rect_width: f32) {
        if chart.bars.is_empty() {
            return;
        }
        let bar_w = rect_width / chart.visible_bars as f32;
        let delta_bars = (dx / bar_w) as isize;
        let new_offset = (chart.view_offset as isize - delta_bars)
            .clamp(0, (chart.bars.len() + CHART_RIGHT_MARGIN) as isize - 1)
            as usize;
        chart.view_offset = new_offset;
    }

    // ── floating window rendering ────────────────────────────────────────────
}
