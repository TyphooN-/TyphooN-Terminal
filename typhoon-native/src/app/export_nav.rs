use super::*;

impl TyphooNApp {
    pub(super) fn close_all_windows(&mut self) {
        self.show_settings = false;
        self.show_risk_calc = false;
        self.show_compound_calc = false;
        self.show_ai_chat = false;
        self.show_claude_code = false;
        self.show_antigravity_cli = false;
        self.show_codex_cli = false;
        self.show_hermes_cli = false;
        self.show_grok_cli = false;
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
        self.show_sma_intelligence = false;
        self.show_fundamentals = false;
        self.show_analyst = false;
        self.show_holders = false;
        self.show_orderbook_window = false;
        self.show_cor = false;
        self.show_seasonals = false;
        self.show_montecarlo = false;
        self.show_stress_test = false;
        self.show_volume_profile = false;
        self.show_order_flow = false;
        self.show_bookmap = false;
        self.bookmap_windows.clear();
        self.show_outliers = false;
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
        self.show_help = false;
        self.show_connect = false;
        self.show_indicators_panel = false;
        self.show_data_window = false;
        self.show_company_info_window = false;
        self.show_alerts = false;
        self.show_scrape_status = false;
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
        // Keep the camera authoritative. Rebuilding from rounded legacy fields
        // here makes body-drag free-look visibly snap back after scroll zoom.
        chart.zoom_chart_bars_by(factor as f64);
    }
}
