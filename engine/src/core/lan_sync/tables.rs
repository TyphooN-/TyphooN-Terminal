// ── Syncable Tables (whitelist) ────────────────────────────────────

pub(super) const SYNCABLE_TABLES: &[&str] = &[
    "sec_filings",
    "sec_insider_trades",
    "sec_filing_alerts",
    "sec_scrape_index",
    "sec_filing_content",
    "fundamentals",
    "quarterly_financials",
    "institutional_holders",
    "research_news",
    "research_dividends",
    "research_earnings_estimates",
    "research_rating_changes",
    "research_financials",
    "research_executives",
    // ── Round 4 ─────────────────────────────
    "research_stock_splits",
    "research_etf_holdings",
    "research_analyst_recs",
    "research_price_target",
    "research_esg",
    "research_index_members",
    // ── Round 5 ─────────────────────────────
    "research_insider_trades",
    "research_institutional_holders",
    "research_shares_float",
    "research_historical_price",
    "research_earnings_surprise",
    // ── Round 6 ─────────────────────────────
    "research_world_indices",
    "research_market_movers",
    "research_sector_performance",
    "research_wacc",
    // ── Round 7 ─────────────────────────────
    "research_currency_rates",
    "research_beta",
    "research_ddm",
    "research_relative_valuation",
    "research_figi",
    // ── Round 8 ─────────────────────────────
    "research_hra",
    "research_dcf",
    "research_svm",
    "research_options_chain",
    "research_ivol",
    // ── Round 9 ─────────────────────────────
    "research_seasonality",
    "research_correlation",
    "research_total_return",
    "research_technicals",
    "research_vol_skew",
    // ── Round 10 ────────────────────────────
    "research_leverage",
    "research_accruals",
    "research_realized_vol",
    "research_fcf_yield",
    "research_short_interest",
    // ── Round 11 ────────────────────────────
    "research_altman_z",
    "research_piotroski",
    "research_ohlc_vol",
    "research_eps_beat",
    "research_price_target_dispersion",
    // ── Round 12 ────────────────────────────
    "research_insider_activity",
    "research_divg",
    "research_earm",
    "research_sector_rotation",
    "research_updm",
    // ── Round 13 ────────────────────────────
    "research_momentum",
    "research_liquidity",
    "research_breakout",
    "research_cash_cycle",
    "research_credit",
    // ── Round 14 ────────────────────────────
    "research_growm",
    "research_flow",
    "research_regime",
    "research_relvol",
    "research_margins",
    // ── Round 15 ────────────────────────────
    "research_val",
    "research_qual",
    "research_risk",
    "research_insstrk",
    "research_covg",
    // ── Round 16 ────────────────────────────
    "research_vrk",
    "research_qrk",
    "research_rrk",
    "research_relepsgr",
    "research_pead",
    // ── Round 17 ────────────────────────────
    "research_sizef",
    "research_momf",
    "research_peadrank",
    "research_fqm",
    "research_revrank",
    // ── Round 18 ────────────────────────────
    "research_levrank",
    "research_operank",
    "research_fqmrank",
    "research_liqrank",
    "research_surpstk",
    // ── Round 19 ────────────────────────────
    "research_dvdrank",
    "research_earmrank",
    "research_updgrank",
    "research_gy",
    "research_des",
    // ── Round 20 ────────────────────────────
    "research_dvdyieldrank",
    "research_shrank",
    "research_atrann",
    "research_ddhist",
    "research_priceperf",
    // ── Round 21 ────────────────────────────
    "research_betarank",
    "research_pegrank",
    "research_fhighlow",
    "research_rvcone",
    "research_calpb",
    // ── Round 22 ────────────────────────────
    "research_retskew",
    "research_retkurt",
    "research_tailr",
    "research_runlen",
    "research_dayrange",
    // ── web article ingestion ──────────────
    "research_web_articles",
    // ── Round 23 ────────────────────────────
    "research_autocor",
    "research_hurst",
    "research_hitrate",
    "research_glasym",
    "research_volratio",
    // ── Round 24 ────────────────────────────
    "research_drawup",
    "research_gapstats",
    "research_volcluster",
    "research_closeplc",
    "research_mrhl",
    // ── Round 25 ────────────────────────────
    "research_downvol",
    "research_sharpr",
    "research_effratio",
    "research_wickbias",
    "research_volofvol",
    // ── Round 26 ────────────────────────────
    "research_calmar",
    "research_ulcer",
    "research_varratio",
    "research_amihud",
    "research_jbnorm",
    // ── Round 27 ────────────────────────────
    "research_omega",
    "research_dfa",
    "research_burke",
    "research_monthseas",
    "research_rollsprd",
    // ── Round 28 ────────────────────────────
    "research_parkinson",
    "research_gkvol",
    "research_rsvol",
    "research_cvar",
    "research_doweffect",
    // ── Round 29 ────────────────────────────
    "research_sterling",
    "research_kellyf",
    "research_ljungb",
    "research_runstest",
    "research_zeroret",
    // ── Round 30 ────────────────────────────
    "research_psr",
    "research_adf",
    "research_mnkendall",
    "research_bipower",
    "research_dddur",
    // ── Round 31 ────────────────────────────
    "research_hilltail",
    "research_archlm",
    "research_painratio",
    "research_cusum",
    "research_cfvar",
    // ── Round 32 ────────────────────────────
    "research_entropy",
    "research_rachev",
    "research_gpr",
    "research_pacf",
    "research_apen",
    // ── Round 33 ────────────────────────────
    "research_upr",
    "research_levereff",
    "research_drawdar",
    "research_varhalf",
    "research_gini",
    // ── Round 34 ────────────────────────────
    "research_sampen",
    "research_permen",
    "research_recfact",
    "research_kpss",
    "research_specent",
    // ── Round 35 ────────────────────────────
    "research_robvol",
    "research_renyient",
    "research_retquant",
    "research_msent",
    "research_ewmavol",
    // ── Round 36 ────────────────────────────
    "research_ksnorm",
    "research_adtest",
    "research_lmom",
    "research_kylelam",
    "research_peakover",
    // ── Round 37 ────────────────────────────
    "research_higuchi",
    "research_pickands",
    "research_kappa3",
    "research_lyapunov",
    "research_rankac",
    // ── Round 38 ────────────────────────────
    "research_bnsjump",
    "research_pproot",
    "research_mfdfa",
    "research_hillks",
    "research_tsi",
    // ── Round 39 ────────────────────────────
    "research_garch11",
    "research_sadf",
    "research_cordim",
    "research_skspec",
    "research_automi",
    // ── Round 40 ────────────────────────────
    "research_durbinwatson",
    "research_bdstest",
    "research_breuschpagan",
    "research_turnpts",
    "research_periodogram",
    // ── Round 41 ────────────────────────────
    "research_mcleodli",
    "research_oufit",
    "research_gph",
    "research_burgspec",
    "research_kendalltau",
    // ── Round 42 ────────────────────────────
    "research_squeeze",
    "research_squeezerank",
    "research_bbsqueeze",
    "research_donchian",
    "research_kama",
    // ── Round 43 ────────────────────────────
    "research_ichimoku",
    "research_supertrend",
    "research_keltner",
    "research_fisher",
    "research_aroon",
    // ── Round 44 ────────────────────────────
    "research_adx",
    "research_cci",
    "research_cmf",
    "research_mfi",
    "research_psar",
    // ── Round 45 ────────────────────────────
    "research_vortex",
    "research_chop",
    "research_obv",
    "research_trix",
    "research_hma",
    // ── Round 46 ────────────────────────────
    "research_ppo",
    "research_dpo",
    "research_kst",
    "research_ultosc",
    "research_willr",
    // ── Round 47 ────────────────────────────
    "research_mass",
    "research_chaikosc",
    "research_klinger",
    "research_stochrsi",
    "research_awesome",
    // ── Round 48 ────────────────────────────
    "research_efi",
    "research_emv",
    "research_nvi",
    "research_pvi",
    "research_coppock",
    // ── Round 49 ────────────────────────────
    "research_cmo",
    "research_qstick",
    "research_disparity",
    "research_bop",
    "research_schaff",
    // ── Round 50 ────────────────────────────
    "research_stoch",
    "research_macd",
    "research_vwap",
    "research_mcgd",
    "research_rwi",
    // ── Round 51 ────────────────────────────
    "research_dema",
    "research_tema",
    "research_linreg",
    "research_pivots",
    "research_heikin",
    // ── cross-client AI response cache ─────
    "ai_response_cache",
    // ── Round 52 ────────────────────────────
    "research_alma",
    "research_zlema",
    "research_elderray",
    "research_tsf",
    "research_rvi",
    // ── Round 53 ────────────────────────────
    "research_trima",
    "research_t3",
    "research_vidya",
    "research_smi",
    "research_pvt",
    // ── Round 54 ────────────────────────────
    "research_ac",
    "research_chvol",
    "research_bbwidth",
    "research_elderimp",
    "research_rmi",
    // ── Options Expiration Calendar ──────────
    "research_symbol_expirations",
    // ── Round 55 ────────────────────────────
    "research_smma",
    "research_alligator",
    "research_crsi",
    "research_seb",
    "research_imi",
    // ── Round 56 ────────────────────────────
    "research_gmma",
    "research_maenv",
    "research_adl",
    "research_vhf",
    "research_vroc",
    // ── Round 57 ────────────────────────────
    "research_kdj",
    "research_qqe",
    "research_pmo",
    "research_cfo",
    "research_tmf",
    // ── Round 58 ────────────────────────────
    "research_fractals",
    "research_ift_rsi",
    "research_mama",
    "research_cog",
    "research_didi",
    // ── Round 59 ────────────────────────────
    "research_demarker",
    "research_gator",
    "research_bw_mfi",
    "research_vwma",
    "research_stddev",
    // ── Round 60 ────────────────────────────
    "research_wma",
    "research_rainbow",
    "research_mesa_sine",
    "research_frama",
    "research_ibs",
    // ── Round 61 ────────────────────────────
    "research_laguerre_rsi",
    "research_zigzag",
    "research_pgo",
    "research_ht_trendline",
    "research_midpoint",
    // ── Round 62 ────────────────────────────
    "research_mass_index",
    "research_natr",
    "research_ttm_squeeze",
    "research_force_index",
    "research_trange",
    // ── Round 63 ────────────────────────────
    "research_linearreg_slope",
    "research_ht_dcperiod",
    "research_ht_trendmode",
    "research_accbands",
    "research_stochf",
    // ── Round 64 ────────────────────────────
    "research_linearreg",
    "research_linearreg_angle",
    "research_ht_dcphase",
    "research_ht_sine",
    "research_ht_phasor",
    // ── Round 65 ────────────────────────────
    "research_midprice",
    "research_apo",
    "research_mom",
    "research_sarext",
    "research_adxr",
    // ── Round 66 ────────────────────────────
    "research_avgprice",
    "research_medprice",
    "research_typprice",
    "research_wclprice",
    "research_variance",
    // ── Round 67 ────────────────────────────
    "research_plus_di",
    "research_minus_di",
    "research_plus_dm",
    "research_minus_dm",
    "research_dx",
    // ── Round 68 ────────────────────────────
    "research_roc",
    "research_rocp",
    "research_rocr",
    "research_rocr100",
    "research_correl",
    // ── Round 69 ────────────────────────────
    "research_min",
    "research_max",
    "research_minmax",
    "research_minindex",
    "research_maxindex",
    // ── Round 70 ────────────────────────────
    "research_bbands",
    "research_ad",
    "research_adosc",
    "research_sum",
    "research_linreg_intercept",
    // ── Round 71 ────────────────────────────
    "research_aroonosc",
    "research_minmaxindex",
    "research_macdext",
    "research_macdfix",
    "research_mavp",
    // ── Round 72 ────────────────────────────
    "research_cdl_doji",
    "research_cdl_hammer",
    "research_cdl_shooting_star",
    "research_cdl_engulfing",
    "research_cdl_harami",
    // ── Round 73 ────────────────────────────
    "research_cdl_morning_star",
    "research_cdl_evening_star",
    "research_cdl_three_black_crows",
    "research_cdl_three_white_soldiers",
    "research_cdl_dark_cloud_cover",
    // ── Round 74 ────────────────────────────
    "research_cdl_piercing",
    "research_cdl_dragonfly_doji",
    "research_cdl_gravestone_doji",
    "research_cdl_hanging_man",
    "research_cdl_inverted_hammer",
    // ── Round 75 ────────────────────────────
    "research_cdl_harami_cross",
    "research_cdl_long_legged_doji",
    "research_cdl_marubozu",
    "research_cdl_spinning_top",
    "research_cdl_tristar",
    // ── Round 76 ────────────────────────────
    "research_cdl_doji_star",
    "research_cdl_morning_doji_star",
    "research_cdl_evening_doji_star",
    "research_cdl_abandoned_baby",
    "research_cdl_three_inside",
    // ── Round 77 ────────────────────────────
    "research_cdl_belt_hold",
    "research_cdl_closing_marubozu",
    "research_cdl_high_wave",
    "research_cdl_long_line",
    "research_cdl_short_line",
    // ── Round 78 ────────────────────────────
    "research_cdl_counterattack",
    "research_cdl_homing_pigeon",
    "research_cdl_in_neck",
    "research_cdl_on_neck",
    "research_cdl_thrusting",
    // ── Round 79 ────────────────────────────
    "research_cdl_two_crows",
    "research_cdl_three_line_strike",
    "research_cdl_three_outside",
    "research_cdl_matching_low",
    // ── Round 80 ────────────────────────────
    "research_cdl_separating_lines",
    "research_cdl_stick_sandwich",
    "research_cdl_rickshaw_man",
    "research_cdl_takuri",
    // ── Round 81/82 ────────────────────────
    "research_cdl_three_stars_in_south",
    "research_cdl_identical_three_crows",
    "research_cdl_kicking",
    "research_cdl_kicking_by_length",
    "research_cdl_ladder_bottom",
    "research_cdl_unique_three_river",
    // ── Round 83/84 ────────────────────────
    "research_cdl_advance_block",
    "research_cdl_breakaway",
    "research_cdl_gap_side_side_white",
    "research_cdl_upside_gap_two_crows",
    "research_cdl_xside_gap_three_methods",
    "research_cdl_conceal_baby_swallow",
    // ── Round 85/86 ────────────────────────
    "research_cdl_hikkake",
    "research_cdl_hikkake_mod",
    "research_cdl_mat_hold",
    "research_cdl_rise_fall_three_methods",
    // ── Round 87/88 ────────────────────────
    "research_cdl_stalled_pattern",
    "research_cdl_tasuki_gap",
    // ── Round 89/90 ────────────────────────
    "research_momrank_multi",
    "research_corrstk",
    // ── Round 91/92 ────────────────────────
    "research_tlrank",
    "research_corrrank",
    // ── Round 93/94 ────────────────────────
    "research_operank_delta",
    "research_divacc",
    "research_epsacc",
    "research_vrp",
    // ── Round 95 ───────────────────────────
    "research_short_interest_history",
    "research_shortrank_delta",
    // ── Round 96 ───────────────────────────
    "research_insiderconc",
    // ── Round 76 ────────────────────────────
    "research_modsharpe",
    "research_hsiehtest",
    "research_chowbreak",
    "research_driftburst",
    "research_hlvclust",
    // ── Round 77 ────────────────────────────
    "research_yangzhang",
    "research_kuiper",
    "research_dagostino",
    "research_baiperron",
    "research_kupiecpof",
];

/// Returns the CREATE TABLE statement for a syncable table (whitelist only).
pub(super) fn create_table_sql(table: &str) -> Option<&'static str> {
    match table {
        "sec_filings" => Some(
            "CREATE TABLE IF NOT EXISTS sec_filings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ticker TEXT NOT NULL,
                form_type TEXT NOT NULL,
                accession_number TEXT UNIQUE NOT NULL,
                filing_date TEXT NOT NULL,
                url TEXT NOT NULL,
                company_name TEXT DEFAULT '',
                importance_score INTEGER DEFAULT 50,
                category TEXT DEFAULT 'OTHER',
                summary TEXT DEFAULT '',
                insider_flag BOOLEAN DEFAULT FALSE,
                created_at INTEGER NOT NULL
            )",
        ),
        "sec_insider_trades" => Some(
            "CREATE TABLE IF NOT EXISTS sec_insider_trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ticker TEXT NOT NULL,
                accession_number TEXT NOT NULL,
                insider_name TEXT NOT NULL,
                insider_title TEXT DEFAULT '',
                transaction_date TEXT NOT NULL,
                transaction_type TEXT NOT NULL,
                shares REAL DEFAULT 0,
                price REAL DEFAULT 0,
                aggregate_value REAL DEFAULT 0,
                is_officer BOOLEAN DEFAULT FALSE,
                is_director BOOLEAN DEFAULT FALSE,
                created_at INTEGER NOT NULL
            )",
        ),
        "sec_filing_alerts" => Some(
            "CREATE TABLE IF NOT EXISTS sec_filing_alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ticker TEXT NOT NULL,
                alert_type TEXT NOT NULL,
                message TEXT NOT NULL,
                filing_accession TEXT,
                importance INTEGER DEFAULT 50,
                created_at INTEGER NOT NULL,
                dismissed BOOLEAN DEFAULT FALSE,
                dismissed_reason TEXT
            )",
        ),
        "sec_scrape_index" => Some(
            "CREATE TABLE IF NOT EXISTS sec_scrape_index (
                ticker TEXT PRIMARY KEY,
                last_scrape_date TEXT,
                filing_count INTEGER DEFAULT 0,
                cik TEXT,
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "sec_filing_content" => Some(
            "CREATE TABLE IF NOT EXISTS sec_filing_content (
                accession_number TEXT PRIMARY KEY,
                content_plain TEXT NOT NULL,
                content_size INTEGER DEFAULT 0,
                fetched_at INTEGER NOT NULL
            )",
        ),
        "fundamentals" => Some(
            "CREATE TABLE IF NOT EXISTS fundamentals (
                symbol TEXT PRIMARY KEY,
                cik TEXT,
                company_name TEXT NOT NULL DEFAULT '',
                sector TEXT NOT NULL DEFAULT '',
                industry TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                market_cap REAL,
                enterprise_value REAL,
                total_debt REAL,
                cash_and_equivalents REAL,
                shares_outstanding REAL,
                stock_price REAL,
                mcap_ev_ratio REAL,
                next_earnings_date TEXT,
                previous_earnings_date TEXT,
                next_ex_dividend_date TEXT,
                next_dividend_payment_date TEXT,
                last_dividend_payment_date TEXT,
                is_dividend_stock INTEGER NOT NULL DEFAULT 0,
                dividend_yield REAL,
                pe_ratio REAL,
                forward_pe REAL,
                peg_ratio REAL,
                price_to_book REAL,
                price_to_sales REAL,
                ev_to_ebitda REAL,
                profit_margin REAL,
                operating_margin REAL,
                roe REAL,
                roa REAL,
                beta REAL,
                short_ratio REAL,
                short_percent_of_float REAL,
                last_updated TEXT NOT NULL DEFAULT '',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "quarterly_financials" => Some(
            "CREATE TABLE IF NOT EXISTS quarterly_financials (
                symbol TEXT NOT NULL,
                period_end TEXT NOT NULL,
                total_revenue REAL,
                net_income REAL,
                free_cash_flow REAL,
                gross_profit REAL,
                operating_income REAL,
                ebitda REAL,
                eps REAL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (symbol, period_end)
            )",
        ),
        "institutional_holders" => Some(
            "CREATE TABLE IF NOT EXISTS institutional_holders (
                symbol TEXT NOT NULL,
                holder_name TEXT NOT NULL,
                shares INTEGER NOT NULL DEFAULT 0,
                pct_held REAL NOT NULL DEFAULT 0.0,
                value REAL NOT NULL DEFAULT 0.0,
                date_reported TEXT NOT NULL DEFAULT '',
                updated_at INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (symbol, holder_name)
            )",
        ),
        "research_news" => Some(
            "CREATE TABLE IF NOT EXISTS research_news (
                url_hash TEXT PRIMARY KEY,
                symbol TEXT NOT NULL DEFAULT '',
                source TEXT NOT NULL DEFAULT '',
                provider TEXT NOT NULL DEFAULT '',
                headline TEXT NOT NULL DEFAULT '',
                summary TEXT NOT NULL DEFAULT '',
                url TEXT NOT NULL DEFAULT '',
                published_at INTEGER NOT NULL DEFAULT 0,
                image_url TEXT NOT NULL DEFAULT '',
                sentiment TEXT NOT NULL DEFAULT '',
                sentiment_score REAL NOT NULL DEFAULT 0.0,
                tickers_json TEXT NOT NULL DEFAULT '[]',
                categories_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dividends" => Some(
            "CREATE TABLE IF NOT EXISTS research_dividends (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_earnings_estimates" => Some(
            "CREATE TABLE IF NOT EXISTS research_earnings_estimates (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rating_changes" => Some(
            "CREATE TABLE IF NOT EXISTS research_rating_changes (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_financials" => Some(
            "CREATE TABLE IF NOT EXISTS research_financials (
                symbol TEXT PRIMARY KEY,
                bundle_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_executives" => Some(
            "CREATE TABLE IF NOT EXISTS research_executives (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_stock_splits" => Some(
            "CREATE TABLE IF NOT EXISTS research_stock_splits (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_etf_holdings" => Some(
            "CREATE TABLE IF NOT EXISTS research_etf_holdings (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_analyst_recs" => Some(
            "CREATE TABLE IF NOT EXISTS research_analyst_recs (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_price_target" => Some(
            "CREATE TABLE IF NOT EXISTS research_price_target (
                symbol TEXT PRIMARY KEY,
                target_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_esg" => Some(
            "CREATE TABLE IF NOT EXISTS research_esg (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_index_members" => Some(
            "CREATE TABLE IF NOT EXISTS research_index_members (
                index_code TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_insider_trades" => Some(
            "CREATE TABLE IF NOT EXISTS research_insider_trades (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_institutional_holders" => Some(
            "CREATE TABLE IF NOT EXISTS research_institutional_holders (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_shares_float" => Some(
            "CREATE TABLE IF NOT EXISTS research_shares_float (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_historical_price" => Some(
            "CREATE TABLE IF NOT EXISTS research_historical_price (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_earnings_surprise" => Some(
            "CREATE TABLE IF NOT EXISTS research_earnings_surprise (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 6 ─────────────────────────────
        "research_world_indices" => Some(
            "CREATE TABLE IF NOT EXISTS research_world_indices (
                snapshot_key TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_market_movers" => Some(
            "CREATE TABLE IF NOT EXISTS research_market_movers (
                snapshot_key TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_sector_performance" => Some(
            "CREATE TABLE IF NOT EXISTS research_sector_performance (
                snapshot_key TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_wacc" => Some(
            "CREATE TABLE IF NOT EXISTS research_wacc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 7 ─────────────────────────────
        "research_currency_rates" => Some(
            "CREATE TABLE IF NOT EXISTS research_currency_rates (
                snapshot_key TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_beta" => Some(
            "CREATE TABLE IF NOT EXISTS research_beta (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ddm" => Some(
            "CREATE TABLE IF NOT EXISTS research_ddm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_relative_valuation" => Some(
            "CREATE TABLE IF NOT EXISTS research_relative_valuation (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_figi" => Some(
            "CREATE TABLE IF NOT EXISTS research_figi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 8 ─────────────────────────────
        "research_hra" => Some(
            "CREATE TABLE IF NOT EXISTS research_hra (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dcf" => Some(
            "CREATE TABLE IF NOT EXISTS research_dcf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_svm" => Some(
            "CREATE TABLE IF NOT EXISTS research_svm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_options_chain" => Some(
            "CREATE TABLE IF NOT EXISTS research_options_chain (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ivol" => Some(
            "CREATE TABLE IF NOT EXISTS research_ivol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 9 ─────────────────────────────
        "research_seasonality" => Some(
            "CREATE TABLE IF NOT EXISTS research_seasonality (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_correlation" => Some(
            "CREATE TABLE IF NOT EXISTS research_correlation (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_total_return" => Some(
            "CREATE TABLE IF NOT EXISTS research_total_return (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_technicals" => Some(
            "CREATE TABLE IF NOT EXISTS research_technicals (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_vol_skew" => Some(
            "CREATE TABLE IF NOT EXISTS research_vol_skew (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_leverage" => Some(
            "CREATE TABLE IF NOT EXISTS research_leverage (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_accruals" => Some(
            "CREATE TABLE IF NOT EXISTS research_accruals (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_realized_vol" => Some(
            "CREATE TABLE IF NOT EXISTS research_realized_vol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_fcf_yield" => Some(
            "CREATE TABLE IF NOT EXISTS research_fcf_yield (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_short_interest" => Some(
            "CREATE TABLE IF NOT EXISTS research_short_interest (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_altman_z" => Some(
            "CREATE TABLE IF NOT EXISTS research_altman_z (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_piotroski" => Some(
            "CREATE TABLE IF NOT EXISTS research_piotroski (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ohlc_vol" => Some(
            "CREATE TABLE IF NOT EXISTS research_ohlc_vol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_eps_beat" => Some(
            "CREATE TABLE IF NOT EXISTS research_eps_beat (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_price_target_dispersion" => Some(
            "CREATE TABLE IF NOT EXISTS research_price_target_dispersion (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_insider_activity" => Some(
            "CREATE TABLE IF NOT EXISTS research_insider_activity (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_divg" => Some(
            "CREATE TABLE IF NOT EXISTS research_divg (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_earm" => Some(
            "CREATE TABLE IF NOT EXISTS research_earm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_sector_rotation" => Some(
            "CREATE TABLE IF NOT EXISTS research_sector_rotation (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_updm" => Some(
            "CREATE TABLE IF NOT EXISTS research_updm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_momentum" => Some(
            "CREATE TABLE IF NOT EXISTS research_momentum (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_liquidity" => Some(
            "CREATE TABLE IF NOT EXISTS research_liquidity (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_breakout" => Some(
            "CREATE TABLE IF NOT EXISTS research_breakout (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cash_cycle" => Some(
            "CREATE TABLE IF NOT EXISTS research_cash_cycle (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_credit" => Some(
            "CREATE TABLE IF NOT EXISTS research_credit (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_growm" => Some(
            "CREATE TABLE IF NOT EXISTS research_growm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_flow" => Some(
            "CREATE TABLE IF NOT EXISTS research_flow (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_regime" => Some(
            "CREATE TABLE IF NOT EXISTS research_regime (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_relvol" => Some(
            "CREATE TABLE IF NOT EXISTS research_relvol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_margins" => Some(
            "CREATE TABLE IF NOT EXISTS research_margins (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_val" => Some(
            "CREATE TABLE IF NOT EXISTS research_val (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_qual" => Some(
            "CREATE TABLE IF NOT EXISTS research_qual (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_risk" => Some(
            "CREATE TABLE IF NOT EXISTS research_risk (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_insstrk" => Some(
            "CREATE TABLE IF NOT EXISTS research_insstrk (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_covg" => Some(
            "CREATE TABLE IF NOT EXISTS research_covg (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_vrk" => Some(
            "CREATE TABLE IF NOT EXISTS research_vrk (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_qrk" => Some(
            "CREATE TABLE IF NOT EXISTS research_qrk (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rrk" => Some(
            "CREATE TABLE IF NOT EXISTS research_rrk (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_relepsgr" => Some(
            "CREATE TABLE IF NOT EXISTS research_relepsgr (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_pead" => Some(
            "CREATE TABLE IF NOT EXISTS research_pead (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_sizef" => Some(
            "CREATE TABLE IF NOT EXISTS research_sizef (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_momf" => Some(
            "CREATE TABLE IF NOT EXISTS research_momf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_peadrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_peadrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_fqm" => Some(
            "CREATE TABLE IF NOT EXISTS research_fqm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_revrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_revrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_levrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_levrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_operank" => Some(
            "CREATE TABLE IF NOT EXISTS research_operank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_fqmrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_fqmrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_liqrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_liqrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_surpstk" => Some(
            "CREATE TABLE IF NOT EXISTS research_surpstk (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dvdrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_dvdrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_earmrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_earmrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_updgrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_updgrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_gy" => Some(
            "CREATE TABLE IF NOT EXISTS research_gy (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_des" => Some(
            "CREATE TABLE IF NOT EXISTS research_des (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dvdyieldrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_dvdyieldrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_shrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_shrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_atrann" => Some(
            "CREATE TABLE IF NOT EXISTS research_atrann (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ddhist" => Some(
            "CREATE TABLE IF NOT EXISTS research_ddhist (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_priceperf" => Some(
            "CREATE TABLE IF NOT EXISTS research_priceperf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_betarank" => Some(
            "CREATE TABLE IF NOT EXISTS research_betarank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_pegrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_pegrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_fhighlow" => Some(
            "CREATE TABLE IF NOT EXISTS research_fhighlow (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rvcone" => Some(
            "CREATE TABLE IF NOT EXISTS research_rvcone (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_calpb" => Some(
            "CREATE TABLE IF NOT EXISTS research_calpb (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_retskew" => Some(
            "CREATE TABLE IF NOT EXISTS research_retskew (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_retkurt" => Some(
            "CREATE TABLE IF NOT EXISTS research_retkurt (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_tailr" => Some(
            "CREATE TABLE IF NOT EXISTS research_tailr (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_runlen" => Some(
            "CREATE TABLE IF NOT EXISTS research_runlen (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dayrange" => Some(
            "CREATE TABLE IF NOT EXISTS research_dayrange (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── web article ingestion ──
        "research_web_articles" => Some(
            "CREATE TABLE IF NOT EXISTS research_web_articles (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 23 ──
        "research_autocor" => Some(
            "CREATE TABLE IF NOT EXISTS research_autocor (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_hurst" => Some(
            "CREATE TABLE IF NOT EXISTS research_hurst (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_hitrate" => Some(
            "CREATE TABLE IF NOT EXISTS research_hitrate (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_glasym" => Some(
            "CREATE TABLE IF NOT EXISTS research_glasym (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_volratio" => Some(
            "CREATE TABLE IF NOT EXISTS research_volratio (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 24 ──
        "research_drawup" => Some(
            "CREATE TABLE IF NOT EXISTS research_drawup (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_gapstats" => Some(
            "CREATE TABLE IF NOT EXISTS research_gapstats (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_volcluster" => Some(
            "CREATE TABLE IF NOT EXISTS research_volcluster (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_closeplc" => Some(
            "CREATE TABLE IF NOT EXISTS research_closeplc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_mrhl" => Some(
            "CREATE TABLE IF NOT EXISTS research_mrhl (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 25 ──
        "research_downvol" => Some(
            "CREATE TABLE IF NOT EXISTS research_downvol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_sharpr" => Some(
            "CREATE TABLE IF NOT EXISTS research_sharpr (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_effratio" => Some(
            "CREATE TABLE IF NOT EXISTS research_effratio (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_wickbias" => Some(
            "CREATE TABLE IF NOT EXISTS research_wickbias (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_volofvol" => Some(
            "CREATE TABLE IF NOT EXISTS research_volofvol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 26 ──
        "research_calmar" => Some(
            "CREATE TABLE IF NOT EXISTS research_calmar (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ulcer" => Some(
            "CREATE TABLE IF NOT EXISTS research_ulcer (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_varratio" => Some(
            "CREATE TABLE IF NOT EXISTS research_varratio (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_amihud" => Some(
            "CREATE TABLE IF NOT EXISTS research_amihud (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_jbnorm" => Some(
            "CREATE TABLE IF NOT EXISTS research_jbnorm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 27 ──
        "research_omega" => Some(
            "CREATE TABLE IF NOT EXISTS research_omega (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dfa" => Some(
            "CREATE TABLE IF NOT EXISTS research_dfa (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_burke" => Some(
            "CREATE TABLE IF NOT EXISTS research_burke (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_monthseas" => Some(
            "CREATE TABLE IF NOT EXISTS research_monthseas (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rollsprd" => Some(
            "CREATE TABLE IF NOT EXISTS research_rollsprd (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 28 ──
        "research_parkinson" => Some(
            "CREATE TABLE IF NOT EXISTS research_parkinson (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_gkvol" => Some(
            "CREATE TABLE IF NOT EXISTS research_gkvol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rsvol" => Some(
            "CREATE TABLE IF NOT EXISTS research_rsvol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cvar" => Some(
            "CREATE TABLE IF NOT EXISTS research_cvar (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_doweffect" => Some(
            "CREATE TABLE IF NOT EXISTS research_doweffect (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 29 ──
        "research_sterling" => Some(
            "CREATE TABLE IF NOT EXISTS research_sterling (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_kellyf" => Some(
            "CREATE TABLE IF NOT EXISTS research_kellyf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ljungb" => Some(
            "CREATE TABLE IF NOT EXISTS research_ljungb (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_runstest" => Some(
            "CREATE TABLE IF NOT EXISTS research_runstest (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_zeroret" => Some(
            "CREATE TABLE IF NOT EXISTS research_zeroret (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 30 ──
        "research_psr" => Some(
            "CREATE TABLE IF NOT EXISTS research_psr (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_adf" => Some(
            "CREATE TABLE IF NOT EXISTS research_adf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_mnkendall" => Some(
            "CREATE TABLE IF NOT EXISTS research_mnkendall (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_bipower" => Some(
            "CREATE TABLE IF NOT EXISTS research_bipower (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dddur" => Some(
            "CREATE TABLE IF NOT EXISTS research_dddur (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 31 ──
        "research_hilltail" => Some(
            "CREATE TABLE IF NOT EXISTS research_hilltail (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_archlm" => Some(
            "CREATE TABLE IF NOT EXISTS research_archlm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_painratio" => Some(
            "CREATE TABLE IF NOT EXISTS research_painratio (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cusum" => Some(
            "CREATE TABLE IF NOT EXISTS research_cusum (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cfvar" => Some(
            "CREATE TABLE IF NOT EXISTS research_cfvar (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 32 ──
        "research_entropy" => Some(
            "CREATE TABLE IF NOT EXISTS research_entropy (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rachev" => Some(
            "CREATE TABLE IF NOT EXISTS research_rachev (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_gpr" => Some(
            "CREATE TABLE IF NOT EXISTS research_gpr (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_pacf" => Some(
            "CREATE TABLE IF NOT EXISTS research_pacf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_apen" => Some(
            "CREATE TABLE IF NOT EXISTS research_apen (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 33 ──
        "research_upr" => Some(
            "CREATE TABLE IF NOT EXISTS research_upr (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_levereff" => Some(
            "CREATE TABLE IF NOT EXISTS research_levereff (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_drawdar" => Some(
            "CREATE TABLE IF NOT EXISTS research_drawdar (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_varhalf" => Some(
            "CREATE TABLE IF NOT EXISTS research_varhalf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_gini" => Some(
            "CREATE TABLE IF NOT EXISTS research_gini (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 34 ──
        "research_sampen" => Some(
            "CREATE TABLE IF NOT EXISTS research_sampen (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_permen" => Some(
            "CREATE TABLE IF NOT EXISTS research_permen (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_recfact" => Some(
            "CREATE TABLE IF NOT EXISTS research_recfact (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_kpss" => Some(
            "CREATE TABLE IF NOT EXISTS research_kpss (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_specent" => Some(
            "CREATE TABLE IF NOT EXISTS research_specent (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 35 ──
        "research_robvol" => Some(
            "CREATE TABLE IF NOT EXISTS research_robvol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_renyient" => Some(
            "CREATE TABLE IF NOT EXISTS research_renyient (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_retquant" => Some(
            "CREATE TABLE IF NOT EXISTS research_retquant (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_msent" => Some(
            "CREATE TABLE IF NOT EXISTS research_msent (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ewmavol" => Some(
            "CREATE TABLE IF NOT EXISTS research_ewmavol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 36 ──
        "research_ksnorm" => Some(
            "CREATE TABLE IF NOT EXISTS research_ksnorm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_adtest" => Some(
            "CREATE TABLE IF NOT EXISTS research_adtest (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_lmom" => Some(
            "CREATE TABLE IF NOT EXISTS research_lmom (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_kylelam" => Some(
            "CREATE TABLE IF NOT EXISTS research_kylelam (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_peakover" => Some(
            "CREATE TABLE IF NOT EXISTS research_peakover (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 37 ──
        "research_higuchi" => Some(
            "CREATE TABLE IF NOT EXISTS research_higuchi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_pickands" => Some(
            "CREATE TABLE IF NOT EXISTS research_pickands (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_kappa3" => Some(
            "CREATE TABLE IF NOT EXISTS research_kappa3 (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_lyapunov" => Some(
            "CREATE TABLE IF NOT EXISTS research_lyapunov (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rankac" => Some(
            "CREATE TABLE IF NOT EXISTS research_rankac (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 38 ──
        "research_bnsjump" => Some(
            "CREATE TABLE IF NOT EXISTS research_bnsjump (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_pproot" => Some(
            "CREATE TABLE IF NOT EXISTS research_pproot (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_mfdfa" => Some(
            "CREATE TABLE IF NOT EXISTS research_mfdfa (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_hillks" => Some(
            "CREATE TABLE IF NOT EXISTS research_hillks (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_tsi" => Some(
            "CREATE TABLE IF NOT EXISTS research_tsi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 39 ──
        "research_garch11" => Some(
            "CREATE TABLE IF NOT EXISTS research_garch11 (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_sadf" => Some(
            "CREATE TABLE IF NOT EXISTS research_sadf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cordim" => Some(
            "CREATE TABLE IF NOT EXISTS research_cordim (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_skspec" => Some(
            "CREATE TABLE IF NOT EXISTS research_skspec (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_automi" => Some(
            "CREATE TABLE IF NOT EXISTS research_automi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 40 ──
        "research_durbinwatson" => Some(
            "CREATE TABLE IF NOT EXISTS research_durbinwatson (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_bdstest" => Some(
            "CREATE TABLE IF NOT EXISTS research_bdstest (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_breuschpagan" => Some(
            "CREATE TABLE IF NOT EXISTS research_breuschpagan (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_turnpts" => Some(
            "CREATE TABLE IF NOT EXISTS research_turnpts (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_periodogram" => Some(
            "CREATE TABLE IF NOT EXISTS research_periodogram (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 41 ──
        "research_mcleodli" => Some(
            "CREATE TABLE IF NOT EXISTS research_mcleodli (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_oufit" => Some(
            "CREATE TABLE IF NOT EXISTS research_oufit (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_gph" => Some(
            "CREATE TABLE IF NOT EXISTS research_gph (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_burgspec" => Some(
            "CREATE TABLE IF NOT EXISTS research_burgspec (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_kendalltau" => Some(
            "CREATE TABLE IF NOT EXISTS research_kendalltau (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 42 ──
        "research_squeeze" => Some(
            "CREATE TABLE IF NOT EXISTS research_squeeze (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_squeezerank" => Some(
            "CREATE TABLE IF NOT EXISTS research_squeezerank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_bbsqueeze" => Some(
            "CREATE TABLE IF NOT EXISTS research_bbsqueeze (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_donchian" => Some(
            "CREATE TABLE IF NOT EXISTS research_donchian (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_kama" => Some(
            "CREATE TABLE IF NOT EXISTS research_kama (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 43 ──
        "research_ichimoku" => Some(
            "CREATE TABLE IF NOT EXISTS research_ichimoku (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_supertrend" => Some(
            "CREATE TABLE IF NOT EXISTS research_supertrend (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_keltner" => Some(
            "CREATE TABLE IF NOT EXISTS research_keltner (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_fisher" => Some(
            "CREATE TABLE IF NOT EXISTS research_fisher (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_aroon" => Some(
            "CREATE TABLE IF NOT EXISTS research_aroon (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 44 ──
        "research_adx" => Some(
            "CREATE TABLE IF NOT EXISTS research_adx (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cci" => Some(
            "CREATE TABLE IF NOT EXISTS research_cci (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cmf" => Some(
            "CREATE TABLE IF NOT EXISTS research_cmf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_mfi" => Some(
            "CREATE TABLE IF NOT EXISTS research_mfi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_psar" => Some(
            "CREATE TABLE IF NOT EXISTS research_psar (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 45 ──
        "research_vortex" => Some(
            "CREATE TABLE IF NOT EXISTS research_vortex (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_chop" => Some(
            "CREATE TABLE IF NOT EXISTS research_chop (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_obv" => Some(
            "CREATE TABLE IF NOT EXISTS research_obv (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_trix" => Some(
            "CREATE TABLE IF NOT EXISTS research_trix (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_hma" => Some(
            "CREATE TABLE IF NOT EXISTS research_hma (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 46 ──
        "research_ppo" => Some(
            "CREATE TABLE IF NOT EXISTS research_ppo (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dpo" => Some(
            "CREATE TABLE IF NOT EXISTS research_dpo (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_kst" => Some(
            "CREATE TABLE IF NOT EXISTS research_kst (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ultosc" => Some(
            "CREATE TABLE IF NOT EXISTS research_ultosc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_willr" => Some(
            "CREATE TABLE IF NOT EXISTS research_willr (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 47 ──
        "research_mass" => Some(
            "CREATE TABLE IF NOT EXISTS research_mass (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_chaikosc" => Some(
            "CREATE TABLE IF NOT EXISTS research_chaikosc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_klinger" => Some(
            "CREATE TABLE IF NOT EXISTS research_klinger (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_stochrsi" => Some(
            "CREATE TABLE IF NOT EXISTS research_stochrsi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_awesome" => Some(
            "CREATE TABLE IF NOT EXISTS research_awesome (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_efi" => Some(
            "CREATE TABLE IF NOT EXISTS research_efi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_emv" => Some(
            "CREATE TABLE IF NOT EXISTS research_emv (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_nvi" => Some(
            "CREATE TABLE IF NOT EXISTS research_nvi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_pvi" => Some(
            "CREATE TABLE IF NOT EXISTS research_pvi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_coppock" => Some(
            "CREATE TABLE IF NOT EXISTS research_coppock (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cmo" => Some(
            "CREATE TABLE IF NOT EXISTS research_cmo (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_qstick" => Some(
            "CREATE TABLE IF NOT EXISTS research_qstick (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_disparity" => Some(
            "CREATE TABLE IF NOT EXISTS research_disparity (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_bop" => Some(
            "CREATE TABLE IF NOT EXISTS research_bop (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_schaff" => Some(
            "CREATE TABLE IF NOT EXISTS research_schaff (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_stoch" => Some(
            "CREATE TABLE IF NOT EXISTS research_stoch (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_macd" => Some(
            "CREATE TABLE IF NOT EXISTS research_macd (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_vwap" => Some(
            "CREATE TABLE IF NOT EXISTS research_vwap (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_mcgd" => Some(
            "CREATE TABLE IF NOT EXISTS research_mcgd (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rwi" => Some(
            "CREATE TABLE IF NOT EXISTS research_rwi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dema" => Some(
            "CREATE TABLE IF NOT EXISTS research_dema (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_tema" => Some(
            "CREATE TABLE IF NOT EXISTS research_tema (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_linreg" => Some(
            "CREATE TABLE IF NOT EXISTS research_linreg (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_pivots" => Some(
            "CREATE TABLE IF NOT EXISTS research_pivots (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_heikin" => Some(
            "CREATE TABLE IF NOT EXISTS research_heikin (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── cross-client AI response cache ──
        "ai_response_cache" => Some(
            "CREATE TABLE IF NOT EXISTS ai_response_cache (
                prompt_hash TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                prompt_preview TEXT NOT NULL DEFAULT '',
                response TEXT NOT NULL,
                token_count_prompt INTEGER NOT NULL DEFAULT 0,
                token_count_completion INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                hit_count INTEGER NOT NULL DEFAULT 0,
                source_client TEXT NOT NULL DEFAULT ''
            )",
        ),
        // ── Round 52 ────────────────────────
        "research_alma" => Some(
            "CREATE TABLE IF NOT EXISTS research_alma (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_zlema" => Some(
            "CREATE TABLE IF NOT EXISTS research_zlema (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_elderray" => Some(
            "CREATE TABLE IF NOT EXISTS research_elderray (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_tsf" => Some(
            "CREATE TABLE IF NOT EXISTS research_tsf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rvi" => Some(
            "CREATE TABLE IF NOT EXISTS research_rvi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 53 ────────────────────────
        "research_trima" => Some(
            "CREATE TABLE IF NOT EXISTS research_trima (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_t3" => Some(
            "CREATE TABLE IF NOT EXISTS research_t3 (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_vidya" => Some(
            "CREATE TABLE IF NOT EXISTS research_vidya (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_smi" => Some(
            "CREATE TABLE IF NOT EXISTS research_smi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_pvt" => Some(
            "CREATE TABLE IF NOT EXISTS research_pvt (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 54 ────────────────────────
        "research_ac" => Some(
            "CREATE TABLE IF NOT EXISTS research_ac (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_chvol" => Some(
            "CREATE TABLE IF NOT EXISTS research_chvol (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_bbwidth" => Some(
            "CREATE TABLE IF NOT EXISTS research_bbwidth (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_elderimp" => Some(
            "CREATE TABLE IF NOT EXISTS research_elderimp (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rmi" => Some(
            "CREATE TABLE IF NOT EXISTS research_rmi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_symbol_expirations" => Some(
            "CREATE TABLE IF NOT EXISTS research_symbol_expirations (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_smma" => Some(
            "CREATE TABLE IF NOT EXISTS research_smma (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_alligator" => Some(
            "CREATE TABLE IF NOT EXISTS research_alligator (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_crsi" => Some(
            "CREATE TABLE IF NOT EXISTS research_crsi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_seb" => Some(
            "CREATE TABLE IF NOT EXISTS research_seb (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_imi" => Some(
            "CREATE TABLE IF NOT EXISTS research_imi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_gmma" => Some(
            "CREATE TABLE IF NOT EXISTS research_gmma (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_maenv" => Some(
            "CREATE TABLE IF NOT EXISTS research_maenv (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_adl" => Some(
            "CREATE TABLE IF NOT EXISTS research_adl (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_vhf" => Some(
            "CREATE TABLE IF NOT EXISTS research_vhf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_vroc" => Some(
            "CREATE TABLE IF NOT EXISTS research_vroc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_kdj" => Some(
            "CREATE TABLE IF NOT EXISTS research_kdj (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_qqe" => Some(
            "CREATE TABLE IF NOT EXISTS research_qqe (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_pmo" => Some(
            "CREATE TABLE IF NOT EXISTS research_pmo (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cfo" => Some(
            "CREATE TABLE IF NOT EXISTS research_cfo (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_tmf" => Some(
            "CREATE TABLE IF NOT EXISTS research_tmf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_fractals" => Some(
            "CREATE TABLE IF NOT EXISTS research_fractals (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ift_rsi" => Some(
            "CREATE TABLE IF NOT EXISTS research_ift_rsi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_mama" => Some(
            "CREATE TABLE IF NOT EXISTS research_mama (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cog" => Some(
            "CREATE TABLE IF NOT EXISTS research_cog (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_didi" => Some(
            "CREATE TABLE IF NOT EXISTS research_didi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_demarker" => Some(
            "CREATE TABLE IF NOT EXISTS research_demarker (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_gator" => Some(
            "CREATE TABLE IF NOT EXISTS research_gator (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_bw_mfi" => Some(
            "CREATE TABLE IF NOT EXISTS research_bw_mfi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_vwma" => Some(
            "CREATE TABLE IF NOT EXISTS research_vwma (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_stddev" => Some(
            "CREATE TABLE IF NOT EXISTS research_stddev (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_wma" => Some(
            "CREATE TABLE IF NOT EXISTS research_wma (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rainbow" => Some(
            "CREATE TABLE IF NOT EXISTS research_rainbow (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_mesa_sine" => Some(
            "CREATE TABLE IF NOT EXISTS research_mesa_sine (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_frama" => Some(
            "CREATE TABLE IF NOT EXISTS research_frama (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ibs" => Some(
            "CREATE TABLE IF NOT EXISTS research_ibs (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_laguerre_rsi" => Some(
            "CREATE TABLE IF NOT EXISTS research_laguerre_rsi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_zigzag" => Some(
            "CREATE TABLE IF NOT EXISTS research_zigzag (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_pgo" => Some(
            "CREATE TABLE IF NOT EXISTS research_pgo (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ht_trendline" => Some(
            "CREATE TABLE IF NOT EXISTS research_ht_trendline (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_midpoint" => Some(
            "CREATE TABLE IF NOT EXISTS research_midpoint (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 62 ──
        "research_mass_index" => Some(
            "CREATE TABLE IF NOT EXISTS research_mass_index (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_natr" => Some(
            "CREATE TABLE IF NOT EXISTS research_natr (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ttm_squeeze" => Some(
            "CREATE TABLE IF NOT EXISTS research_ttm_squeeze (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_force_index" => Some(
            "CREATE TABLE IF NOT EXISTS research_force_index (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_trange" => Some(
            "CREATE TABLE IF NOT EXISTS research_trange (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 63 ──
        "research_linearreg_slope" => Some(
            "CREATE TABLE IF NOT EXISTS research_linearreg_slope (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ht_dcperiod" => Some(
            "CREATE TABLE IF NOT EXISTS research_ht_dcperiod (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ht_trendmode" => Some(
            "CREATE TABLE IF NOT EXISTS research_ht_trendmode (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_accbands" => Some(
            "CREATE TABLE IF NOT EXISTS research_accbands (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_stochf" => Some(
            "CREATE TABLE IF NOT EXISTS research_stochf (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 64 ──
        "research_linearreg" => Some(
            "CREATE TABLE IF NOT EXISTS research_linearreg (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_linearreg_angle" => Some(
            "CREATE TABLE IF NOT EXISTS research_linearreg_angle (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ht_dcphase" => Some(
            "CREATE TABLE IF NOT EXISTS research_ht_dcphase (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ht_sine" => Some(
            "CREATE TABLE IF NOT EXISTS research_ht_sine (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ht_phasor" => Some(
            "CREATE TABLE IF NOT EXISTS research_ht_phasor (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 65 ──
        "research_midprice" => Some(
            "CREATE TABLE IF NOT EXISTS research_midprice (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_apo" => Some(
            "CREATE TABLE IF NOT EXISTS research_apo (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_mom" => Some(
            "CREATE TABLE IF NOT EXISTS research_mom (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_sarext" => Some(
            "CREATE TABLE IF NOT EXISTS research_sarext (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_adxr" => Some(
            "CREATE TABLE IF NOT EXISTS research_adxr (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 66 ──
        "research_avgprice" => Some(
            "CREATE TABLE IF NOT EXISTS research_avgprice (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_medprice" => Some(
            "CREATE TABLE IF NOT EXISTS research_medprice (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_typprice" => Some(
            "CREATE TABLE IF NOT EXISTS research_typprice (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_wclprice" => Some(
            "CREATE TABLE IF NOT EXISTS research_wclprice (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_variance" => Some(
            "CREATE TABLE IF NOT EXISTS research_variance (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 67 ──
        "research_plus_di" => Some(
            "CREATE TABLE IF NOT EXISTS research_plus_di (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_minus_di" => Some(
            "CREATE TABLE IF NOT EXISTS research_minus_di (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_plus_dm" => Some(
            "CREATE TABLE IF NOT EXISTS research_plus_dm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_minus_dm" => Some(
            "CREATE TABLE IF NOT EXISTS research_minus_dm (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dx" => Some(
            "CREATE TABLE IF NOT EXISTS research_dx (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 68 ──
        "research_roc" => Some(
            "CREATE TABLE IF NOT EXISTS research_roc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rocp" => Some(
            "CREATE TABLE IF NOT EXISTS research_rocp (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rocr" => Some(
            "CREATE TABLE IF NOT EXISTS research_rocr (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_rocr100" => Some(
            "CREATE TABLE IF NOT EXISTS research_rocr100 (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_correl" => Some(
            "CREATE TABLE IF NOT EXISTS research_correl (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 69 ──
        "research_min" => Some(
            "CREATE TABLE IF NOT EXISTS research_min (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_max" => Some(
            "CREATE TABLE IF NOT EXISTS research_max (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_minmax" => Some(
            "CREATE TABLE IF NOT EXISTS research_minmax (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_minindex" => Some(
            "CREATE TABLE IF NOT EXISTS research_minindex (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_maxindex" => Some(
            "CREATE TABLE IF NOT EXISTS research_maxindex (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 70 ──
        "research_bbands" => Some(
            "CREATE TABLE IF NOT EXISTS research_bbands (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_ad" => Some(
            "CREATE TABLE IF NOT EXISTS research_ad (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_adosc" => Some(
            "CREATE TABLE IF NOT EXISTS research_adosc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_sum" => Some(
            "CREATE TABLE IF NOT EXISTS research_sum (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_linreg_intercept" => Some(
            "CREATE TABLE IF NOT EXISTS research_linreg_intercept (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        // ── Round 71 ──
        "research_aroonosc" => Some(
            "CREATE TABLE IF NOT EXISTS research_aroonosc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_minmaxindex" => Some(
            "CREATE TABLE IF NOT EXISTS research_minmaxindex (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_macdext" => Some(
            "CREATE TABLE IF NOT EXISTS research_macdext (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_macdfix" => Some(
            "CREATE TABLE IF NOT EXISTS research_macdfix (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_mavp" => Some(
            "CREATE TABLE IF NOT EXISTS research_mavp (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_doji" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_doji (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_hammer" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_hammer (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_shooting_star" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_shooting_star (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_engulfing" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_engulfing (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_harami" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_harami (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_morning_star" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_morning_star (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_evening_star" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_evening_star (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_three_black_crows" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_three_black_crows (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_three_white_soldiers" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_three_white_soldiers (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_dark_cloud_cover" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_dark_cloud_cover (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_piercing" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_piercing (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_dragonfly_doji" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_dragonfly_doji (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_gravestone_doji" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_gravestone_doji (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_hanging_man" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_hanging_man (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_inverted_hammer" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_inverted_hammer (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_harami_cross" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_harami_cross (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_long_legged_doji" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_long_legged_doji (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_marubozu" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_marubozu (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_spinning_top" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_spinning_top (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_tristar" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_tristar (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_doji_star" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_doji_star (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_morning_doji_star" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_morning_doji_star (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_evening_doji_star" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_evening_doji_star (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_abandoned_baby" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_abandoned_baby (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_three_inside" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_three_inside (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_belt_hold" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_belt_hold (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_closing_marubozu" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_closing_marubozu (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_high_wave" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_high_wave (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_long_line" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_long_line (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_short_line" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_short_line (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_counterattack" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_counterattack (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_homing_pigeon" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_homing_pigeon (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_in_neck" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_in_neck (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_on_neck" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_on_neck (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_thrusting" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_thrusting (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_two_crows" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_two_crows (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_three_line_strike" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_three_line_strike (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_three_outside" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_three_outside (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_matching_low" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_matching_low (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_separating_lines" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_separating_lines (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_stick_sandwich" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_stick_sandwich (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_rickshaw_man" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_rickshaw_man (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_takuri" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_takuri (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_three_stars_in_south" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_three_stars_in_south (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_identical_three_crows" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_identical_three_crows (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_kicking" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_kicking (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_kicking_by_length" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_kicking_by_length (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_ladder_bottom" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_ladder_bottom (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_unique_three_river" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_unique_three_river (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_advance_block" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_advance_block (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_breakaway" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_breakaway (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_gap_side_side_white" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_gap_side_side_white (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_upside_gap_two_crows" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_upside_gap_two_crows (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_xside_gap_three_methods" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_xside_gap_three_methods (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_conceal_baby_swallow" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_conceal_baby_swallow (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_hikkake" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_hikkake (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_hikkake_mod" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_hikkake_mod (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_mat_hold" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_mat_hold (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_rise_fall_three_methods" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_rise_fall_three_methods (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_stalled_pattern" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_stalled_pattern (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_cdl_tasuki_gap" => Some(
            "CREATE TABLE IF NOT EXISTS research_cdl_tasuki_gap (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_momrank_multi" => Some(
            "CREATE TABLE IF NOT EXISTS research_momrank_multi (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_corrstk" => Some(
            "CREATE TABLE IF NOT EXISTS research_corrstk (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_tlrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_tlrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_corrrank" => Some(
            "CREATE TABLE IF NOT EXISTS research_corrrank (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_operank_delta" => Some(
            "CREATE TABLE IF NOT EXISTS research_operank_delta (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_divacc" => Some(
            "CREATE TABLE IF NOT EXISTS research_divacc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_epsacc" => Some(
            "CREATE TABLE IF NOT EXISTS research_epsacc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_vrp" => Some(
            "CREATE TABLE IF NOT EXISTS research_vrp (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_short_interest_history" => Some(
            "CREATE TABLE IF NOT EXISTS research_short_interest_history (
                symbol TEXT PRIMARY KEY,
                rows_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_shortrank_delta" => Some(
            "CREATE TABLE IF NOT EXISTS research_shortrank_delta (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_insiderconc" => Some(
            "CREATE TABLE IF NOT EXISTS research_insiderconc (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_modsharpe" => Some(
            "CREATE TABLE IF NOT EXISTS research_modsharpe (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_hsiehtest" => Some(
            "CREATE TABLE IF NOT EXISTS research_hsiehtest (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_chowbreak" => Some(
            "CREATE TABLE IF NOT EXISTS research_chowbreak (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_driftburst" => Some(
            "CREATE TABLE IF NOT EXISTS research_driftburst (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_hlvclust" => Some(
            "CREATE TABLE IF NOT EXISTS research_hlvclust (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_yangzhang" => Some(
            "CREATE TABLE IF NOT EXISTS research_yangzhang (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_kuiper" => Some(
            "CREATE TABLE IF NOT EXISTS research_kuiper (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_dagostino" => Some(
            "CREATE TABLE IF NOT EXISTS research_dagostino (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_baiperron" => Some(
            "CREATE TABLE IF NOT EXISTS research_baiperron (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        "research_kupiecpof" => Some(
            "CREATE TABLE IF NOT EXISTS research_kupiecpof (
                symbol TEXT PRIMARY KEY,
                snapshot_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        ),
        _ => None,
    }
}

/// Returns the timestamp column name for incremental sync, if available.
/// Tables without a usable timestamp column return None and fall back to full sync.
pub(super) fn table_timestamp_column(table: &str) -> Option<&'static str> {
    match table {
        "sec_filings" => Some("created_at"),
        "sec_insider_trades" => Some("created_at"),
        "sec_filing_alerts" => Some("created_at"),
        "sec_filing_content" => Some("fetched_at"),
        "fundamentals" => Some("updated_at"),
        "quarterly_financials" => Some("updated_at"),
        "institutional_holders" => Some("updated_at"),
        "sec_scrape_index" => Some("updated_at"),
        "research_news" => Some("updated_at"),
        "research_dividends" => Some("updated_at"),
        "research_earnings_estimates" => Some("updated_at"),
        "research_rating_changes" => Some("updated_at"),
        "research_financials" => Some("updated_at"),
        "research_executives" => Some("updated_at"),
        "research_stock_splits" => Some("updated_at"),
        "research_etf_holdings" => Some("updated_at"),
        "research_analyst_recs" => Some("updated_at"),
        "research_price_target" => Some("updated_at"),
        "research_esg" => Some("updated_at"),
        "research_index_members" => Some("updated_at"),
        "research_insider_trades" => Some("updated_at"),
        "research_institutional_holders" => Some("updated_at"),
        "research_shares_float" => Some("updated_at"),
        "research_historical_price" => Some("updated_at"),
        "research_earnings_surprise" => Some("updated_at"),
        "research_world_indices" => Some("updated_at"),
        "research_market_movers" => Some("updated_at"),
        "research_sector_performance" => Some("updated_at"),
        "research_wacc" => Some("updated_at"),
        "research_currency_rates" => Some("updated_at"),
        "research_beta" => Some("updated_at"),
        "research_ddm" => Some("updated_at"),
        "research_relative_valuation" => Some("updated_at"),
        "research_figi" => Some("updated_at"),
        "research_hra" => Some("updated_at"),
        "research_dcf" => Some("updated_at"),
        "research_svm" => Some("updated_at"),
        "research_options_chain" => Some("updated_at"),
        "research_ivol" => Some("updated_at"),
        "research_seasonality" => Some("updated_at"),
        "research_correlation" => Some("updated_at"),
        "research_total_return" => Some("updated_at"),
        "research_technicals" => Some("updated_at"),
        "research_vol_skew" => Some("updated_at"),
        "research_leverage" => Some("updated_at"),
        "research_accruals" => Some("updated_at"),
        "research_realized_vol" => Some("updated_at"),
        "research_fcf_yield" => Some("updated_at"),
        "research_short_interest" => Some("updated_at"),
        "research_altman_z" => Some("updated_at"),
        "research_piotroski" => Some("updated_at"),
        "research_ohlc_vol" => Some("updated_at"),
        "research_eps_beat" => Some("updated_at"),
        "research_price_target_dispersion" => Some("updated_at"),
        "research_insider_activity" => Some("updated_at"),
        "research_divg" => Some("updated_at"),
        "research_earm" => Some("updated_at"),
        "research_sector_rotation" => Some("updated_at"),
        "research_updm" => Some("updated_at"),
        "research_momentum" => Some("updated_at"),
        "research_liquidity" => Some("updated_at"),
        "research_breakout" => Some("updated_at"),
        "research_cash_cycle" => Some("updated_at"),
        "research_credit" => Some("updated_at"),
        "research_growm" => Some("updated_at"),
        "research_flow" => Some("updated_at"),
        "research_regime" => Some("updated_at"),
        "research_relvol" => Some("updated_at"),
        "research_margins" => Some("updated_at"),
        "research_val" => Some("updated_at"),
        "research_qual" => Some("updated_at"),
        "research_risk" => Some("updated_at"),
        "research_insstrk" => Some("updated_at"),
        "research_covg" => Some("updated_at"),
        "research_vrk" => Some("updated_at"),
        "research_qrk" => Some("updated_at"),
        "research_rrk" => Some("updated_at"),
        "research_relepsgr" => Some("updated_at"),
        "research_pead" => Some("updated_at"),
        "research_sizef" => Some("updated_at"),
        "research_momf" => Some("updated_at"),
        "research_peadrank" => Some("updated_at"),
        "research_fqm" => Some("updated_at"),
        "research_revrank" => Some("updated_at"),
        "research_levrank" => Some("updated_at"),
        "research_operank" => Some("updated_at"),
        "research_fqmrank" => Some("updated_at"),
        "research_liqrank" => Some("updated_at"),
        "research_surpstk" => Some("updated_at"),
        "research_dvdrank" => Some("updated_at"),
        "research_earmrank" => Some("updated_at"),
        "research_updgrank" => Some("updated_at"),
        "research_gy" => Some("updated_at"),
        "research_des" => Some("updated_at"),
        "research_dvdyieldrank" => Some("updated_at"),
        "research_shrank" => Some("updated_at"),
        "research_atrann" => Some("updated_at"),
        "research_ddhist" => Some("updated_at"),
        "research_priceperf" => Some("updated_at"),
        "research_betarank" => Some("updated_at"),
        "research_pegrank" => Some("updated_at"),
        "research_fhighlow" => Some("updated_at"),
        "research_rvcone" => Some("updated_at"),
        "research_calpb" => Some("updated_at"),
        "research_retskew" => Some("updated_at"),
        "research_retkurt" => Some("updated_at"),
        "research_tailr" => Some("updated_at"),
        "research_runlen" => Some("updated_at"),
        "research_dayrange" => Some("updated_at"),
        "research_web_articles" => Some("updated_at"),
        "research_autocor" => Some("updated_at"),
        "research_hurst" => Some("updated_at"),
        "research_hitrate" => Some("updated_at"),
        "research_glasym" => Some("updated_at"),
        "research_volratio" => Some("updated_at"),
        "research_drawup" => Some("updated_at"),
        "research_gapstats" => Some("updated_at"),
        "research_volcluster" => Some("updated_at"),
        "research_closeplc" => Some("updated_at"),
        "research_mrhl" => Some("updated_at"),
        "research_downvol" => Some("updated_at"),
        "research_sharpr" => Some("updated_at"),
        "research_effratio" => Some("updated_at"),
        "research_wickbias" => Some("updated_at"),
        "research_volofvol" => Some("updated_at"),
        // ── Round 26 ──
        "research_calmar" => Some("updated_at"),
        "research_ulcer" => Some("updated_at"),
        "research_varratio" => Some("updated_at"),
        "research_amihud" => Some("updated_at"),
        "research_jbnorm" => Some("updated_at"),
        // ── Round 27 ──
        "research_omega" => Some("updated_at"),
        "research_dfa" => Some("updated_at"),
        "research_burke" => Some("updated_at"),
        "research_monthseas" => Some("updated_at"),
        "research_rollsprd" => Some("updated_at"),
        // ── Round 28 ──
        "research_parkinson" => Some("updated_at"),
        "research_gkvol" => Some("updated_at"),
        "research_rsvol" => Some("updated_at"),
        "research_cvar" => Some("updated_at"),
        "research_doweffect" => Some("updated_at"),
        // ── Round 29 ──
        "research_sterling" => Some("updated_at"),
        "research_kellyf" => Some("updated_at"),
        "research_ljungb" => Some("updated_at"),
        "research_runstest" => Some("updated_at"),
        "research_zeroret" => Some("updated_at"),
        // ── Round 30 ──
        "research_psr" => Some("updated_at"),
        "research_adf" => Some("updated_at"),
        "research_mnkendall" => Some("updated_at"),
        "research_bipower" => Some("updated_at"),
        "research_dddur" => Some("updated_at"),
        // ── Round 31 ──
        "research_hilltail" => Some("updated_at"),
        "research_archlm" => Some("updated_at"),
        "research_painratio" => Some("updated_at"),
        "research_cusum" => Some("updated_at"),
        "research_cfvar" => Some("updated_at"),
        // ── Round 32 ──
        "research_entropy" => Some("updated_at"),
        "research_rachev" => Some("updated_at"),
        "research_gpr" => Some("updated_at"),
        "research_pacf" => Some("updated_at"),
        "research_apen" => Some("updated_at"),
        // ── Round 33 ──
        "research_upr" => Some("updated_at"),
        "research_levereff" => Some("updated_at"),
        "research_drawdar" => Some("updated_at"),
        "research_varhalf" => Some("updated_at"),
        "research_gini" => Some("updated_at"),
        // ── Round 34 ──
        "research_sampen" => Some("updated_at"),
        "research_permen" => Some("updated_at"),
        "research_recfact" => Some("updated_at"),
        "research_kpss" => Some("updated_at"),
        "research_specent" => Some("updated_at"),
        // ── Round 35 ──
        "research_robvol" => Some("updated_at"),
        "research_renyient" => Some("updated_at"),
        "research_retquant" => Some("updated_at"),
        "research_msent" => Some("updated_at"),
        "research_ewmavol" => Some("updated_at"),
        // ── Round 36 ──
        "research_ksnorm" => Some("updated_at"),
        "research_adtest" => Some("updated_at"),
        "research_lmom" => Some("updated_at"),
        "research_kylelam" => Some("updated_at"),
        "research_peakover" => Some("updated_at"),
        // ── Round 37 ──
        "research_higuchi" => Some("updated_at"),
        "research_pickands" => Some("updated_at"),
        "research_kappa3" => Some("updated_at"),
        "research_lyapunov" => Some("updated_at"),
        "research_rankac" => Some("updated_at"),
        // ── Round 38 ──
        "research_bnsjump" => Some("updated_at"),
        "research_pproot" => Some("updated_at"),
        "research_mfdfa" => Some("updated_at"),
        "research_hillks" => Some("updated_at"),
        "research_tsi" => Some("updated_at"),
        // ── Round 39 ──
        "research_garch11" => Some("updated_at"),
        "research_sadf" => Some("updated_at"),
        "research_cordim" => Some("updated_at"),
        "research_skspec" => Some("updated_at"),
        "research_automi" => Some("updated_at"),
        // ── Round 40 ──
        "research_durbinwatson" => Some("updated_at"),
        "research_bdstest" => Some("updated_at"),
        "research_breuschpagan" => Some("updated_at"),
        "research_turnpts" => Some("updated_at"),
        "research_periodogram" => Some("updated_at"),
        // ── Round 41 ──
        "research_mcleodli" => Some("updated_at"),
        "research_oufit" => Some("updated_at"),
        "research_gph" => Some("updated_at"),
        "research_burgspec" => Some("updated_at"),
        "research_kendalltau" => Some("updated_at"),
        // ── Round 42 ──
        "research_squeeze" => Some("updated_at"),
        "research_squeezerank" => Some("updated_at"),
        "research_bbsqueeze" => Some("updated_at"),
        "research_donchian" => Some("updated_at"),
        "research_kama" => Some("updated_at"),
        // ── Round 43 ──
        "research_ichimoku" => Some("updated_at"),
        "research_supertrend" => Some("updated_at"),
        "research_keltner" => Some("updated_at"),
        "research_fisher" => Some("updated_at"),
        "research_aroon" => Some("updated_at"),
        // ── Round 44 ──
        "research_adx" => Some("updated_at"),
        "research_cci" => Some("updated_at"),
        "research_cmf" => Some("updated_at"),
        "research_mfi" => Some("updated_at"),
        "research_psar" => Some("updated_at"),
        // ── Round 45 ──
        "research_vortex" => Some("updated_at"),
        "research_chop" => Some("updated_at"),
        "research_obv" => Some("updated_at"),
        "research_trix" => Some("updated_at"),
        "research_hma" => Some("updated_at"),
        // ── Round 46 ──
        "research_ppo" => Some("updated_at"),
        "research_dpo" => Some("updated_at"),
        "research_kst" => Some("updated_at"),
        "research_ultosc" => Some("updated_at"),
        "research_willr" => Some("updated_at"),
        // ── Round 47 ──
        "research_mass" => Some("updated_at"),
        "research_chaikosc" => Some("updated_at"),
        "research_klinger" => Some("updated_at"),
        "research_stochrsi" => Some("updated_at"),
        "research_awesome" => Some("updated_at"),
        // ── Round 48 ──
        "research_efi" => Some("updated_at"),
        "research_emv" => Some("updated_at"),
        "research_nvi" => Some("updated_at"),
        "research_pvi" => Some("updated_at"),
        "research_coppock" => Some("updated_at"),
        // ── Round 49 ──
        "research_cmo" => Some("updated_at"),
        "research_qstick" => Some("updated_at"),
        "research_disparity" => Some("updated_at"),
        "research_bop" => Some("updated_at"),
        "research_schaff" => Some("updated_at"),
        // ── Round 50 ──
        "research_stoch" => Some("updated_at"),
        "research_macd" => Some("updated_at"),
        "research_vwap" => Some("updated_at"),
        "research_mcgd" => Some("updated_at"),
        "research_rwi" => Some("updated_at"),
        // ── Round 51 ──
        "research_dema" => Some("updated_at"),
        "research_tema" => Some("updated_at"),
        "research_linreg" => Some("updated_at"),
        "research_pivots" => Some("updated_at"),
        "research_heikin" => Some("updated_at"),
        // ── cross-client AI response cache ──
        "ai_response_cache" => Some("updated_at"),
        // ── Round 52 ──
        "research_alma" => Some("updated_at"),
        "research_zlema" => Some("updated_at"),
        "research_elderray" => Some("updated_at"),
        "research_tsf" => Some("updated_at"),
        "research_rvi" => Some("updated_at"),
        // ── Round 53 ──
        "research_trima" => Some("updated_at"),
        "research_t3" => Some("updated_at"),
        "research_vidya" => Some("updated_at"),
        "research_smi" => Some("updated_at"),
        "research_pvt" => Some("updated_at"),
        // ── Round 54 ──
        "research_ac" => Some("updated_at"),
        "research_chvol" => Some("updated_at"),
        "research_bbwidth" => Some("updated_at"),
        "research_elderimp" => Some("updated_at"),
        "research_rmi" => Some("updated_at"),
        "research_symbol_expirations" => Some("updated_at"),
        // ── Round 55 ──
        "research_smma" => Some("updated_at"),
        "research_alligator" => Some("updated_at"),
        "research_crsi" => Some("updated_at"),
        "research_seb" => Some("updated_at"),
        "research_imi" => Some("updated_at"),
        // ── Round 56 ──
        "research_gmma" => Some("updated_at"),
        "research_maenv" => Some("updated_at"),
        "research_adl" => Some("updated_at"),
        "research_vhf" => Some("updated_at"),
        "research_vroc" => Some("updated_at"),
        // ── Round 57 ──
        "research_kdj" => Some("updated_at"),
        "research_qqe" => Some("updated_at"),
        "research_pmo" => Some("updated_at"),
        "research_cfo" => Some("updated_at"),
        "research_tmf" => Some("updated_at"),
        // ── Round 58 ──
        "research_fractals" => Some("updated_at"),
        "research_ift_rsi" => Some("updated_at"),
        "research_mama" => Some("updated_at"),
        "research_cog" => Some("updated_at"),
        "research_didi" => Some("updated_at"),
        // ── Round 59 ──
        "research_demarker" => Some("updated_at"),
        "research_gator" => Some("updated_at"),
        "research_bw_mfi" => Some("updated_at"),
        "research_vwma" => Some("updated_at"),
        "research_stddev" => Some("updated_at"),
        // ── Round 60 ──
        "research_wma" => Some("updated_at"),
        "research_rainbow" => Some("updated_at"),
        "research_mesa_sine" => Some("updated_at"),
        "research_frama" => Some("updated_at"),
        "research_ibs" => Some("updated_at"),
        // ── Round 61 ──
        "research_laguerre_rsi" => Some("updated_at"),
        "research_zigzag" => Some("updated_at"),
        "research_pgo" => Some("updated_at"),
        "research_ht_trendline" => Some("updated_at"),
        "research_midpoint" => Some("updated_at"),
        // ── Round 62 ──
        "research_mass_index" => Some("updated_at"),
        "research_natr" => Some("updated_at"),
        "research_ttm_squeeze" => Some("updated_at"),
        "research_force_index" => Some("updated_at"),
        "research_trange" => Some("updated_at"),
        // ── Round 63 ──
        "research_linearreg_slope" => Some("updated_at"),
        "research_ht_dcperiod" => Some("updated_at"),
        "research_ht_trendmode" => Some("updated_at"),
        "research_accbands" => Some("updated_at"),
        "research_stochf" => Some("updated_at"),
        // ── Round 64 ──
        "research_linearreg" => Some("updated_at"),
        "research_linearreg_angle" => Some("updated_at"),
        "research_ht_dcphase" => Some("updated_at"),
        "research_ht_sine" => Some("updated_at"),
        "research_ht_phasor" => Some("updated_at"),
        // ── Round 65 ──
        "research_midprice" => Some("updated_at"),
        "research_apo" => Some("updated_at"),
        "research_mom" => Some("updated_at"),
        "research_sarext" => Some("updated_at"),
        "research_adxr" => Some("updated_at"),
        // ── Round 66 ──
        "research_avgprice" => Some("updated_at"),
        "research_medprice" => Some("updated_at"),
        "research_typprice" => Some("updated_at"),
        "research_wclprice" => Some("updated_at"),
        "research_variance" => Some("updated_at"),
        // ── Round 67 ──
        "research_plus_di" => Some("updated_at"),
        "research_minus_di" => Some("updated_at"),
        "research_plus_dm" => Some("updated_at"),
        "research_minus_dm" => Some("updated_at"),
        "research_dx" => Some("updated_at"),
        // ── Round 68 ──
        "research_roc" => Some("updated_at"),
        "research_rocp" => Some("updated_at"),
        "research_rocr" => Some("updated_at"),
        "research_rocr100" => Some("updated_at"),
        "research_correl" => Some("updated_at"),
        // ── Round 69 ──
        "research_min" => Some("updated_at"),
        "research_max" => Some("updated_at"),
        "research_minmax" => Some("updated_at"),
        "research_minindex" => Some("updated_at"),
        "research_maxindex" => Some("updated_at"),
        // ── Round 70 ──
        "research_bbands" => Some("updated_at"),
        "research_ad" => Some("updated_at"),
        "research_adosc" => Some("updated_at"),
        "research_sum" => Some("updated_at"),
        "research_linreg_intercept" => Some("updated_at"),
        // ── Round 71 ──
        "research_aroonosc" => Some("updated_at"),
        "research_minmaxindex" => Some("updated_at"),
        "research_macdext" => Some("updated_at"),
        "research_macdfix" => Some("updated_at"),
        "research_mavp" => Some("updated_at"),
        // ── Round 72 ──
        "research_cdl_doji" => Some("updated_at"),
        "research_cdl_hammer" => Some("updated_at"),
        "research_cdl_shooting_star" => Some("updated_at"),
        "research_cdl_engulfing" => Some("updated_at"),
        "research_cdl_harami" => Some("updated_at"),
        // ── Round 73 ──
        "research_cdl_morning_star" => Some("updated_at"),
        "research_cdl_evening_star" => Some("updated_at"),
        "research_cdl_three_black_crows" => Some("updated_at"),
        "research_cdl_three_white_soldiers" => Some("updated_at"),
        "research_cdl_dark_cloud_cover" => Some("updated_at"),
        // ── Round 74 ──
        "research_cdl_piercing" => Some("updated_at"),
        "research_cdl_dragonfly_doji" => Some("updated_at"),
        "research_cdl_gravestone_doji" => Some("updated_at"),
        "research_cdl_hanging_man" => Some("updated_at"),
        "research_cdl_inverted_hammer" => Some("updated_at"),
        // ── Round 75 ──
        "research_cdl_harami_cross" => Some("updated_at"),
        "research_cdl_long_legged_doji" => Some("updated_at"),
        "research_cdl_marubozu" => Some("updated_at"),
        "research_cdl_spinning_top" => Some("updated_at"),
        "research_cdl_tristar" => Some("updated_at"),
        // ── Round 76 ──
        "research_cdl_doji_star" => Some("updated_at"),
        "research_cdl_morning_doji_star" => Some("updated_at"),
        "research_cdl_evening_doji_star" => Some("updated_at"),
        "research_cdl_abandoned_baby" => Some("updated_at"),
        "research_cdl_three_inside" => Some("updated_at"),
        // ── Round 77 ──
        "research_cdl_belt_hold" => Some("updated_at"),
        "research_cdl_closing_marubozu" => Some("updated_at"),
        "research_cdl_high_wave" => Some("updated_at"),
        "research_cdl_long_line" => Some("updated_at"),
        "research_cdl_short_line" => Some("updated_at"),
        // ── Round 78 ──
        "research_cdl_counterattack" => Some("updated_at"),
        "research_cdl_homing_pigeon" => Some("updated_at"),
        "research_cdl_in_neck" => Some("updated_at"),
        "research_cdl_on_neck" => Some("updated_at"),
        "research_cdl_thrusting" => Some("updated_at"),
        // ── Round 79 ──
        "research_cdl_two_crows" => Some("updated_at"),
        "research_cdl_three_line_strike" => Some("updated_at"),
        "research_cdl_three_outside" => Some("updated_at"),
        "research_cdl_matching_low" => Some("updated_at"),
        // ── Round 80 ──
        "research_cdl_separating_lines" => Some("updated_at"),
        "research_cdl_stick_sandwich" => Some("updated_at"),
        "research_cdl_rickshaw_man" => Some("updated_at"),
        "research_cdl_takuri" => Some("updated_at"),
        // ── Round 81/82 ──
        "research_cdl_three_stars_in_south" => Some("updated_at"),
        "research_cdl_identical_three_crows" => Some("updated_at"),
        "research_cdl_kicking" => Some("updated_at"),
        "research_cdl_kicking_by_length" => Some("updated_at"),
        "research_cdl_ladder_bottom" => Some("updated_at"),
        "research_cdl_unique_three_river" => Some("updated_at"),
        // ── Round 83/84 ──
        "research_cdl_advance_block" => Some("updated_at"),
        "research_cdl_breakaway" => Some("updated_at"),
        "research_cdl_gap_side_side_white" => Some("updated_at"),
        "research_cdl_upside_gap_two_crows" => Some("updated_at"),
        "research_cdl_xside_gap_three_methods" => Some("updated_at"),
        "research_cdl_conceal_baby_swallow" => Some("updated_at"),
        // ── Round 85/86 ──
        "research_cdl_hikkake" => Some("updated_at"),
        "research_cdl_hikkake_mod" => Some("updated_at"),
        "research_cdl_mat_hold" => Some("updated_at"),
        "research_cdl_rise_fall_three_methods" => Some("updated_at"),
        // ── Round 87/88 ──
        "research_cdl_stalled_pattern" => Some("updated_at"),
        "research_cdl_tasuki_gap" => Some("updated_at"),
        // ── Round 89/90 ──
        "research_momrank_multi" => Some("updated_at"),
        "research_corrstk" => Some("updated_at"),
        // ── Round 91/92 ──
        "research_tlrank" => Some("updated_at"),
        "research_corrrank" => Some("updated_at"),
        // ── Round 93/94 ──
        "research_operank_delta" => Some("updated_at"),
        "research_divacc" => Some("updated_at"),
        "research_epsacc" => Some("updated_at"),
        "research_vrp" => Some("updated_at"),
        // ── Round 95 ──
        "research_short_interest_history" => Some("updated_at"),
        "research_shortrank_delta" => Some("updated_at"),
        // ── Round 96 ──
        "research_insiderconc" => Some("updated_at"),
        // ── Round 76 ──
        "research_modsharpe" => Some("updated_at"),
        "research_hsiehtest" => Some("updated_at"),
        "research_chowbreak" => Some("updated_at"),
        "research_driftburst" => Some("updated_at"),
        "research_hlvclust" => Some("updated_at"),
        // ── Round 77 ──
        "research_yangzhang" => Some("updated_at"),
        "research_kuiper" => Some("updated_at"),
        "research_dagostino" => Some("updated_at"),
        "research_baiperron" => Some("updated_at"),
        "research_kupiecpof" => Some("updated_at"),
        _ => None,
    }
}
